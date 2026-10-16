use hashbrown::HashMap;

/// Errors produced while parsing an NMEA 0183 sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NmeaError {
    /// The sentence is too short to contain a talker id, sentence type and checksum.
    TooShort,
    /// The sentence does not start with '$' (talker sentences) or '!' (encapsulated sentences, e.g. AIS).
    InvalidStartChar,
    /// The sentence does not contain a checksum ("*XX").
    MissingChecksum,
    /// The trailing checksum is malformed or does not match the sentence contents.
    InvalidChecksum,
    /// The two characters after the start char are not a known talker id.
    UnknownTalkerId,
    /// The talker id is known but the sentence type is not supported.
    UnsupportedSentenceType,
}

impl std::fmt::Display for NmeaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            NmeaError::TooShort => "sentence is too short",
            NmeaError::InvalidStartChar => "sentence does not start with '$' or '!'",
            NmeaError::MissingChecksum => "sentence has no checksum",
            NmeaError::InvalidChecksum => "sentence checksum is invalid",
            NmeaError::UnknownTalkerId => "unknown talker id",
            NmeaError::UnsupportedSentenceType => "unsupported sentence type",
        };
        write!(f, "{message}")
    }
}

impl std::error::Error for NmeaError {}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TalkerId {
    AiAlarmIndicator,
    ApAutoPilot,
    BdBeidouChina,
    CdDsc,
    EcEcdis,
    GaGalileoPs,
    GbBeidouChina,
    GiNavicIrnssIndia,
    GlGlonassIEIC611621,
    GnMultipleSatelliteSystem,
    GpGlobalPositioningSystemReceiver,
    GqQZSSRegionalGpsASJapan,
    HcHeadingCompass,
    HeGyroNorthSeeking,
    IiIntegratedInstrumentation,
    InIntegratedNavigation,
    LcLorancReceiver,
    PqQuectelQuirk,
    QzQzssRegionalGpsASJapan,
    RaRadarARPA,
    SdDepthSounder,
    StSkytraq,
    TiTurnIndicator,
    YxTransducer,
    WiWeatherInstrument,
    #[default]
    NotRecognized,
}

impl TalkerId {
    pub fn as_str(&self) -> &'static str {
        match self {
            TalkerId::AiAlarmIndicator => "AI",
            TalkerId::ApAutoPilot => "AP",
            TalkerId::BdBeidouChina => "BD",
            TalkerId::CdDsc => "CD",
            TalkerId::EcEcdis => "EC",
            TalkerId::GaGalileoPs => "GA",
            TalkerId::GbBeidouChina => "GB",
            TalkerId::GiNavicIrnssIndia => "GI",
            TalkerId::GlGlonassIEIC611621 => "GL",
            TalkerId::GnMultipleSatelliteSystem => "GN",
            TalkerId::GpGlobalPositioningSystemReceiver => "GP",
            TalkerId::GqQZSSRegionalGpsASJapan => "GQ",
            TalkerId::HcHeadingCompass => "HC",
            TalkerId::HeGyroNorthSeeking => "HE",
            TalkerId::IiIntegratedInstrumentation => "II",
            TalkerId::InIntegratedNavigation => "IN",
            TalkerId::LcLorancReceiver => "LC",
            TalkerId::PqQuectelQuirk => "PQ",
            TalkerId::QzQzssRegionalGpsASJapan => "QZ",
            TalkerId::RaRadarARPA => "RA",
            TalkerId::SdDepthSounder => "SD",
            TalkerId::StSkytraq => "ST",
            TalkerId::TiTurnIndicator => "TI",
            TalkerId::YxTransducer => "YX",
            TalkerId::WiWeatherInstrument => "WI",
            TalkerId::NotRecognized => "",
        }
    }

    fn from_prefix(prefix: &str) -> Self {
        match prefix {
            "AI" => TalkerId::AiAlarmIndicator,
            "AP" => TalkerId::ApAutoPilot,
            "BD" => TalkerId::BdBeidouChina,
            "CD" => TalkerId::CdDsc,
            "EC" => TalkerId::EcEcdis,
            "GA" => TalkerId::GaGalileoPs,
            "GB" => TalkerId::GbBeidouChina,
            "GI" => TalkerId::GiNavicIrnssIndia,
            "GL" => TalkerId::GlGlonassIEIC611621,
            "GN" => TalkerId::GnMultipleSatelliteSystem,
            "GP" => TalkerId::GpGlobalPositioningSystemReceiver,
            "GQ" => TalkerId::GqQZSSRegionalGpsASJapan,
            "HC" => TalkerId::HcHeadingCompass,
            "HE" => TalkerId::HeGyroNorthSeeking,
            "II" => TalkerId::IiIntegratedInstrumentation,
            "IN" => TalkerId::InIntegratedNavigation,
            "LC" => TalkerId::LcLorancReceiver,
            "PQ" => TalkerId::PqQuectelQuirk,
            "QZ" => TalkerId::QzQzssRegionalGpsASJapan,
            "RA" => TalkerId::RaRadarARPA,
            "SD" => TalkerId::SdDepthSounder,
            "ST" => TalkerId::StSkytraq,
            "TI" => TalkerId::TiTurnIndicator,
            "YX" => TalkerId::YxTransducer,
            "WI" => TalkerId::WiWeatherInstrument,
            _ => TalkerId::NotRecognized,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SentenceType {
    // Actual vessel heading in degrees true produced by any device or system producing true heading.
    Hdt,
    // AIS messages (encapsulated sentences).
    Vdm,
    Vdo,
    // This ones are sentences commonly emitted by GPS units.
    // Time, Position and fix related data for a GPS receiver.
    Gga,
    Gll,
    Rmc,
    Zda,
    // Depth of water
    Dpt,
    // Water speed and heading
    Vhw,
    // Tracked target message
    Ttm,
    // Target latitude and longitude
    Tll,
    #[default]
    NotRecognized,
}

impl SentenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SentenceType::Hdt => "HDT",
            SentenceType::Vdm => "VDM",
            SentenceType::Vdo => "VDO",
            SentenceType::Gga => "GGA",
            SentenceType::Gll => "GLL",
            SentenceType::Rmc => "RMC",
            SentenceType::Zda => "ZDA",
            SentenceType::Dpt => "DPT",
            SentenceType::Vhw => "VHW",
            SentenceType::Ttm => "TTM",
            SentenceType::Tll => "TLL",
            SentenceType::NotRecognized => "",
        }
    }
}

/// A parsed NMEA 0183 sentence.
///
/// The checksum is validated during [`Nmea::parse`] and is not part of [`Nmea::sentence_fields`].
#[derive(Debug)]
pub struct Nmea {
    /// The leading character of the sentence: '$' for talker sentences, '!' for encapsulated sentences (AIS).
    pub start_char: char,
    pub talker_id: TalkerId,
    pub sentence_type: SentenceType,
    pub sentence_fields: hashbrown::HashMap<String, String>,
}

impl Default for Nmea {
    fn default() -> Self {
        Nmea {
            start_char: '$',
            talker_id: TalkerId::NotRecognized,
            sentence_type: SentenceType::NotRecognized,
            sentence_fields: hashbrown::HashMap::new(),
        }
    }
}

impl Nmea {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&self) {
        println!("\n|----[ NMEA SENTENCE ]");
        println!("|----[ TALKER ID : {}]", self.str_talker_id());
        println!("|----[ SENTENCE TYPE : {}]", self.str_sentence_type());
        self.sentence_fields
            .iter()
            .for_each(|(key, value)| println!("|----[ {} -- {} ]", key, value));
        println!("\n")
    }

    /// Validates the sentence frame (start char + checksum) and parses talker id, sentence type and fields.
    pub fn parse(&mut self, sentence: &str) -> Result<(), NmeaError> {
        let body = validate_checksum(sentence)?;

        let talker_prefix = body.get(1..3).ok_or(NmeaError::TooShort)?;
        let talker_id = TalkerId::from_prefix(talker_prefix);
        if talker_id == TalkerId::NotRecognized {
            return Err(NmeaError::UnknownTalkerId);
        }

        let sentence_prefix = body.get(3..6).ok_or(NmeaError::TooShort)?;
        let (sentence_type, sentence_fields) = self.parse_sentence(body, sentence_prefix);
        if sentence_type == SentenceType::NotRecognized {
            return Err(NmeaError::UnsupportedSentenceType);
        }

        self.start_char = sentence.chars().next().unwrap_or('$');
        self.talker_id = talker_id;
        self.sentence_type = sentence_type;
        self.sentence_fields = sentence_fields;
        Ok(())
    }

    pub fn str_talker_id(&self) -> &'static str {
        self.talker_id.as_str()
    }

    pub fn str_sentence_type(&self) -> &'static str {
        self.sentence_type.as_str()
    }

    /// The leading character of the last successfully parsed sentence.
    pub fn sentence_start(&self) -> char {
        self.start_char
    }

    fn parse_sentence(
        &self,
        sentence: &str,
        sentence_prefix: &str,
    ) -> (SentenceType, hashbrown::HashMap<String, String>) {
        match sentence_prefix {
            "HDT" => (SentenceType::Hdt, self.parse_hdt(sentence)),
            "VDM" => (SentenceType::Vdm, self.parse_ais(sentence)),
            "VDO" => (SentenceType::Vdo, self.parse_ais(sentence)),
            "GGA" => (SentenceType::Gga, self.parse_gga(sentence)),
            "GLL" => (SentenceType::Gll, self.parse_gll(sentence)),
            "RMC" => (SentenceType::Rmc, self.parse_rmc(sentence)),
            "DPT" => (SentenceType::Dpt, self.parse_dpt(sentence)),
            "VHW" => (SentenceType::Vhw, self.parse_vhw(sentence)),
            "TTM" => (SentenceType::Ttm, self.parse_ttm(sentence)),
            "TLL" => (SentenceType::Tll, self.parse_tll(sentence)),
            "ZDA" => (SentenceType::Zda, self.parse_zda(sentence)),
            _ => (SentenceType::NotRecognized, hashbrown::HashMap::new()),
        }
    }

    /// Maps the comma separated fields after the sentence id onto `keys`.
    /// The checksum has already been stripped by [`Nmea::parse`].
    fn fields_from_parts(sentence: &str, keys: &[&str]) -> hashbrown::HashMap<String, String> {
        let parts: Vec<&str> = sentence.split(',').map(str::trim).collect();
        if parts.len() < keys.len() + 1 {
            return HashMap::new();
        }
        keys.iter()
            .zip(parts[1..].iter())
            .map(|(key, value)| (String::from(*key), String::from(*value)))
            .collect()
    }

    fn parse_hdt(&self, sentence: &str) -> hashbrown::HashMap<String, String> {
        Self::fields_from_parts(sentence, &["heading_degrees", "true_indicator"])
    }

    fn parse_ais(&self, sentence: &str) -> hashbrown::HashMap<String, String> {
        Self::fields_from_parts(
            sentence,
            &[
                "total_sentences",
                "sentence_number",
                "sequential_id",
                "channel",
                "payload",
                "fill_bits",
            ],
        )
    }

    fn parse_gga(&self, sentence: &str) -> hashbrown::HashMap<String, String> {
        Self::fields_from_parts(
            sentence,
            &[
                "utc_time",
                "latitude",
                "north_or_south",
                "longitude",
                "east_or_west",
                "gps_quality",
                "number_of_satellites",
                "horizontal_dilution_precision",
                "antenna_altitude",
                "antenna_altitude_units",
                "geoidal_separation",
                "geoidal_separation_units",
                "differential_gps_data_age",
                "differential_reference_station_id",
            ],
        )
    }

    fn parse_gll(&self, sentence: &str) -> hashbrown::HashMap<String, String> {
        Self::fields_from_parts(
            sentence,
            &[
                "latitude",
                "north_or_south",
                "longitude",
                "east_or_west",
                "utc",
                "status_a",
                "faa_mode_indicator",
            ],
        )
    }

    fn parse_rmc(&self, sentence: &str) -> hashbrown::HashMap<String, String> {
        Self::fields_from_parts(
            sentence,
            &[
                "utc",
                "status",
                "latitude",
                "north_or_south",
                "longitude",
                "east_or_west",
                "speed_over_ground",
                "track_made_good",
                "date",
                "magnetic_variation",
                "east_or_west_magnetic",
            ],
        )
    }

    fn parse_dpt(&self, sentence: &str) -> hashbrown::HashMap<String, String> {
        Self::fields_from_parts(
            sentence,
            &["water_depth", "transducer_offset", "max_range_scale_in_use"],
        )
    }

    fn parse_vhw(&self, sentence: &str) -> hashbrown::HashMap<String, String> {
        Self::fields_from_parts(
            sentence,
            &[
                "heading_degrees_true",
                "t_true",
                "heading_degrees_magnetic",
                "m_magnetic",
                "vessel_speed_knots",
                "n_knots",
                "vessel_speed_kmh",
                "k_kilometers",
            ],
        )
    }

    fn parse_ttm(&self, sentence: &str) -> hashbrown::HashMap<String, String> {
        Self::fields_from_parts(
            sentence,
            &[
                "target_number",
                "target_distance",
                "bearing_from_own_ship",
                "true_or_relative",
                "target_speed",
                "target_course",
                "true_or_relative_2",
                "distance_from_closest_poa",
                "time_until_closest_poa",
                "speed_distance_units",
                "target_name",
                "target_status",
                "reference_target",
                "utc_of_data",
                "type_amr",
            ],
        )
    }

    fn parse_tll(&self, sentence: &str) -> hashbrown::HashMap<String, String> {
        Self::fields_from_parts(
            sentence,
            &[
                "target_number",
                "target_latitude",
                "north_or_south",
                "target_longitude",
                "east_or_west",
                "target_name",
                "utc_of_data",
                "status",
                "reference_target",
            ],
        )
    }

    fn parse_zda(&self, sentence: &str) -> hashbrown::HashMap<String, String> {
        Self::fields_from_parts(
            sentence,
            &[
                "utc_time",
                "day",
                "month",
                "year",
                "local_zone_description",
                "local_zone_minutes_description",
            ],
        )
    }
}

/// Checks that `sentence` starts with '$' or '!' and that the trailing "*XX" checksum matches.
/// Returns the sentence body without the checksum on success.
fn validate_checksum(sentence: &str) -> Result<&str, NmeaError> {
    let first_byte = sentence
        .as_bytes()
        .first()
        .copied()
        .ok_or(NmeaError::TooShort)?;
    if first_byte != b'$' && first_byte != b'!' {
        return Err(NmeaError::InvalidStartChar);
    }

    let checksum_separator = sentence.rfind('*').ok_or(NmeaError::MissingChecksum)?;
    let (body, checksum_part) = sentence.split_at(checksum_separator);
    let checksum_str = checksum_part.get(1..3).ok_or(NmeaError::InvalidChecksum)?;
    let expected = u8::from_str_radix(checksum_str, 16).map_err(|_| NmeaError::InvalidChecksum)?;

    let computed = body
        .as_bytes()
        .iter()
        .skip(1)
        .fold(0u8, |acc, byte| acc ^ byte);
    if computed != expected {
        return Err(NmeaError::InvalidChecksum);
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a '$' talker sentence with a correct checksum from `body` (e.g. "IIHDT,33,T").
    fn talker_sentence(body: &str) -> String {
        format!("${}*{:02X}", body, checksum(body))
    }

    /// Builds an encapsulated '!' sentence (AIS) with a correct checksum.
    fn encapsulated_sentence(body: &str) -> String {
        format!("!{}*{:02X}", body, checksum(body))
    }

    fn checksum(body: &str) -> u8 {
        body.bytes().fold(0u8, |acc, byte| acc ^ byte)
    }

    #[test]
    fn parses_valid_hdt_sentence() {
        let mut nmea = Nmea::new();
        nmea.parse(&talker_sentence("IIHDT,33,T")).unwrap();

        assert_eq!(nmea.sentence_start(), '$');
        assert_eq!(nmea.str_talker_id(), "II");
        assert_eq!(nmea.str_sentence_type(), "HDT");
        assert_eq!(
            nmea.sentence_fields
                .get("heading_degrees")
                .map(String::as_str),
            Some("33")
        );
        assert_eq!(
            nmea.sentence_fields
                .get("true_indicator")
                .map(String::as_str),
            Some("T")
        );
    }

    #[test]
    fn parses_known_good_gga_sentence() {
        // Well known example sentence (checksum 0x47).
        let mut nmea = Nmea::new();
        nmea.parse("$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47")
            .unwrap();

        assert_eq!(nmea.str_talker_id(), "GP");
        assert_eq!(nmea.str_sentence_type(), "GGA");
        assert_eq!(
            nmea.sentence_fields.get("utc_time").map(String::as_str),
            Some("123519")
        );
        assert_eq!(
            nmea.sentence_fields.get("latitude").map(String::as_str),
            Some("4807.038")
        );
        assert_eq!(
            nmea.sentence_fields
                .get("north_or_south")
                .map(String::as_str),
            Some("N")
        );
        assert_eq!(
            nmea.sentence_fields.get("longitude").map(String::as_str),
            Some("01131.000")
        );
        assert_eq!(
            nmea.sentence_fields
                .get("number_of_satellites")
                .map(String::as_str),
            Some("08")
        );
    }

    #[test]
    fn parses_rmc_sentence_with_utc_field() {
        let mut nmea = Nmea::new();
        nmea.parse(&talker_sentence(
            "GPRMC,225446,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E",
        ))
        .unwrap();

        // Regression test: this key used to contain a stray comma ("utc,").
        assert_eq!(
            nmea.sentence_fields.get("utc").map(String::as_str),
            Some("225446")
        );
        assert_eq!(
            nmea.sentence_fields.get("status").map(String::as_str),
            Some("A")
        );
        assert_eq!(
            nmea.sentence_fields.get("date").map(String::as_str),
            Some("191194")
        );
    }

    #[test]
    fn parses_ais_vdm_sentence() {
        let mut nmea = Nmea::new();
        nmea.parse(&encapsulated_sentence(
            "AIVDM,1,1,,A,15N:;R0P00PD;88MD5MTDwwD7k,0",
        ))
        .unwrap();

        assert_eq!(nmea.sentence_start(), '!');
        assert_eq!(nmea.str_talker_id(), "AI");
        assert_eq!(nmea.str_sentence_type(), "VDM");
        assert_eq!(
            nmea.sentence_fields.get("channel").map(String::as_str),
            Some("A")
        );
        assert_eq!(
            nmea.sentence_fields.get("payload").map(String::as_str),
            Some("15N:;R0P00PD;88MD5MTDwwD7k")
        );
        assert_eq!(
            nmea.sentence_fields.get("fill_bits").map(String::as_str),
            Some("0")
        );
    }

    #[test]
    fn parses_ais_vdo_sentence() {
        let mut nmea = Nmea::new();
        nmea.parse(&encapsulated_sentence(
            "AIVDO,1,1,,B,15N:;R0P00PD;88MD5MTDwwD7k,0",
        ))
        .unwrap();

        assert_eq!(nmea.str_talker_id(), "AI");
        assert_eq!(nmea.str_sentence_type(), "VDO");
    }

    #[test]
    fn rejects_wrong_checksum() {
        let mut nmea = Nmea::new();
        assert_eq!(
            nmea.parse("$IIHDT,33,T*44"),
            Err(NmeaError::InvalidChecksum)
        );
    }

    #[test]
    fn rejects_missing_checksum() {
        let mut nmea = Nmea::new();
        assert_eq!(nmea.parse("$IIHDT,33,T"), Err(NmeaError::MissingChecksum));
    }

    #[test]
    fn rejects_non_hex_checksum() {
        let mut nmea = Nmea::new();
        assert_eq!(
            nmea.parse("$IIHDT,33,T*ZZ"),
            Err(NmeaError::InvalidChecksum)
        );
    }

    #[test]
    fn rejects_invalid_start_char() {
        let mut nmea = Nmea::new();
        assert_eq!(
            nmea.parse("IIHDT,33,T*0C"),
            Err(NmeaError::InvalidStartChar)
        );
    }

    #[test]
    fn rejects_unknown_talker_id() {
        let mut nmea = Nmea::new();
        assert_eq!(
            nmea.parse(&talker_sentence("ZZHDT,33,T")),
            Err(NmeaError::UnknownTalkerId)
        );
    }

    #[test]
    fn rejects_unsupported_sentence_type() {
        let mut nmea = Nmea::new();
        assert_eq!(
            nmea.parse(&talker_sentence("IIXYZ,1,2,3")),
            Err(NmeaError::UnsupportedSentenceType)
        );
    }

    #[test]
    fn rejects_empty_sentence() {
        let mut nmea = Nmea::new();
        assert_eq!(nmea.parse(""), Err(NmeaError::TooShort));
    }

    #[test]
    fn rejects_short_sentence() {
        let mut nmea = Nmea::new();
        assert_eq!(nmea.parse("$*00"), Err(NmeaError::TooShort));
    }

    #[test]
    fn does_not_panic_on_non_ascii_input() {
        let mut nmea = Nmea::new();
        assert!(nmea.parse("$😀*00").is_err());
        assert!(nmea.parse("!€*00").is_err());
    }
}
