use plotters::prelude::*;

pub struct ShipComponentStats {
    pub total_bytes_sent: f64,
    pub total_bytes_received: f64,
    /// Frames emitted by the component that were dropped by the send policy (non-NMEA traffic,
    /// invalid sentences or prefixes that are not in the policy).
    pub dropped_frames: u64,
    /// (seconds since monitoring started, Mbit/s) samples of the traffic the component sent.
    pub performance_send: Vec<(f64, f64)>,
    /// (seconds since monitoring started, Mbit/s) samples of the traffic delivered to the component.
    pub performance_receive: Vec<(f64, f64)>,
}

impl ShipComponentStats {
    pub fn new() -> Self {
        let mut performance_send: Vec<(f64, f64)> = Vec::new();
        let mut performance_receive: Vec<(f64, f64)> = Vec::new();
        performance_send.push((0.0, 0.0));
        performance_receive.push((0.0, 0.0));

        ShipComponentStats {
            total_bytes_sent: 0.0,
            total_bytes_received: 0.0,
            dropped_frames: 0,
            performance_send,
            performance_receive,
        }
    }

    /// Seconds covered by the recorded samples.
    pub fn elapsed(&self) -> f64 {
        let sent = self
            .performance_send
            .last()
            .map(|(time, _)| *time)
            .unwrap_or(0.0);
        let received = self
            .performance_receive
            .last()
            .map(|(time, _)| *time)
            .unwrap_or(0.0);
        sent.max(received)
    }

    /// Plots both performance series to `./test/imgs/`, scaling the axes to the recorded data.
    pub fn plot_performance(&self, component_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all("./test/imgs")?;

        let max_time = self
            .performance_send
            .iter()
            .chain(self.performance_receive.iter())
            .map(|(time, _)| *time)
            .fold(0.0_f64, f64::max);
        let max_rate = self
            .performance_send
            .iter()
            .chain(self.performance_receive.iter())
            .map(|(_, rate)| *rate)
            .fold(0.0_f64, f64::max);

        // Keep the chart readable when there is little or no traffic.
        let x_limit = max_time.max(1.0);
        let y_limit = (max_rate * 1.1).max(1.0);

        let image_name = format!("./test/imgs/{}-stats.png", component_name);
        let root = BitMapBackend::new(&image_name, (1011, 758)).into_drawing_area();
        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .caption(
                format!("{} performance", component_name),
                ("sans-serif", 20),
            )
            .margin(1)
            .x_label_area_size(50)
            .y_label_area_size(50)
            .build_cartesian_2d(0f64..x_limit, 0f64..y_limit)?;

        chart
            .configure_mesh()
            .x_desc("Time s")
            .y_desc("Bitrate Mbit/s")
            .draw()?;

        chart
            .draw_series(LineSeries::new(self.performance_send.clone(), RED))?
            .label("Sending Mbit/s")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
        chart
            .draw_series(LineSeries::new(self.performance_receive.clone(), BLUE))?
            .label("Receiving Mbit/s")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
        chart
            .configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .draw()?;

        Ok(())
    }
}
