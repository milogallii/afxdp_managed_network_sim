use plotters::prelude::*;

pub struct ShipComponentStats {
    pub total_bytes_sent: f64,
    pub total_bytes_received: f64,
    pub performance_send: Vec<(f64, f64)>,
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
            performance_send,
            performance_receive,
        }
    }

    pub fn plot_performance(&self, component_name: &str) -> Result<(), Box<dyn std::error::Error>> {
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
            .build_cartesian_2d(0f64..30f64, 0f64..4000f64)?;

        chart.configure_mesh().draw()?;

        chart
            .draw_series(LineSeries::new(self.performance_send.clone(), &RED))?
            .label("Sending MBit/s/s")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));
        chart
            .draw_series(LineSeries::new(self.performance_receive.clone(), &BLUE))?
            .label("Receive MBit/s/s")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));
        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .draw()?;

        chart
            .configure_mesh()
            .x_desc("Time s") // Label for the x-axis
            .y_desc("Bitrate Mbit/s") // Label for the y-axis
            .draw()?; // Draw the grid and labels

        Ok(())
    }
}
