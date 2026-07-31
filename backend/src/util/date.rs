//! Minimal date helpers. Deliberately no date/time crate: the whole
//! requirement is "what is today, and what was N days ago", expressed as
//! ISO-8601 date strings, because that is the only date format the upstream
//! APIs and the SQLite columns use.

/// Today, UTC, as `YYYY-MM-DD`.
pub fn today_iso() -> String {
    iso_from_epoch_secs(now_epoch_secs())
}

/// `n` days before today, UTC, as `YYYY-MM-DD`. Used for the rolling
/// measurement windows the fetchers request.
// Not yet called: the rolling-window ingest is a later phase. Kept here
// (with tests) because it belongs next to the algorithm it shares.
#[allow(dead_code)]
pub fn days_ago_iso(n: i64) -> String {
    iso_from_epoch_secs(now_epoch_secs() - n * 86_400)
}

/// Current UTC timestamp as `YYYY-MM-DDTHH:MM:SSZ`, for `last_attempt_at` /
/// `last_success_at` style bookkeeping where a date alone is too coarse.
// Not yet called — consumed by the fetch-checkpoint bookkeeping.
#[allow(dead_code)]
pub fn now_utc_iso() -> String {
    let secs = now_epoch_secs();
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let tod = secs.rem_euclid(86_400);
    let (hh, mm, ss) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// True if `s` is exactly a `YYYY-MM-DD` date.
///
/// This exists because OONI's aggregation API silently switches to hourly
/// granularity on short time windows, returning values like
/// `2026-07-01T06:00:00Z` in the same field that otherwise holds a plain date.
/// That field lands in `blocking_timeline.measurement_date`, which is part of
/// the primary key, so an unvalidated hourly value multiplies rows instead of
/// updating them.
// Not yet called — the guard is wired in with the aggregation-based ingest.
#[allow(dead_code)]
pub fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..].iter().all(u8::is_ascii_digit)
}

fn now_epoch_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn iso_from_epoch_secs(secs: i64) -> String {
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Howard Hinnant's days-since-epoch to civil-date algorithm
/// (http://howardhinnant.github.io/date_algorithms.html).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_from_days_matches_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        // 2024 is a leap year — the day after 02-28 must be 02-29.
        assert_eq!(civil_from_days(19_781), (2024, 2, 28));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(civil_from_days(19_783), (2024, 3, 1));
    }

    #[test]
    fn iso_from_epoch_secs_is_zero_padded() {
        assert_eq!(iso_from_epoch_secs(0), "1970-01-01");
        assert_eq!(iso_from_epoch_secs(19_782 * 86_400), "2024-02-29");
    }

    #[test]
    fn days_ago_is_earlier_than_today() {
        assert!(days_ago_iso(1) < today_iso());
        assert!(days_ago_iso(90) < days_ago_iso(1));
    }

    #[test]
    fn today_and_now_agree_on_the_date_part() {
        assert!(now_utc_iso().starts_with(&today_iso()));
    }

    #[test]
    fn is_iso_date_accepts_dates_and_rejects_timestamps() {
        assert!(is_iso_date("2026-07-31"));
        assert!(is_iso_date("0001-01-01"));
        // The hourly-grain form OONI returns on short windows.
        assert!(!is_iso_date("2026-07-01T06:00:00Z"));
        assert!(!is_iso_date("2026-7-1"));
        assert!(!is_iso_date("2026-07-31 "));
        assert!(!is_iso_date(""));
        assert!(!is_iso_date("not-a-date"));
    }
}
