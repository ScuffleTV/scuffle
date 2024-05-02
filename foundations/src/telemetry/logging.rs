use tracing_subscriber::fmt::time::{ChronoLocal, ChronoUtc, FormatTime};

pub enum TimeFormatter {
	Local(ChronoLocal),
	Utc(ChronoUtc),
	None,
}

impl FormatTime for TimeFormatter {
	fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
		match self {
			TimeFormatter::Local(formatter) => formatter.format_time(w),
			TimeFormatter::Utc(formatter) => formatter.format_time(w),
			TimeFormatter::None => ().format_time(w),
		}
	}
}
