use fe2o3_amqp_types::primitives::Timestamp;
use http::Version;

use crate::error::{InvalidVersion, TryFromProjectedError, TryIntoProjectedError};

pub(crate) fn httpdate_to_timestamp<BE>(
    httpdate: &str
) -> Result<Timestamp, TryIntoProjectedError<BE>> {
    let sys_time = httpdate::parse_http_date(httpdate)?;
    let duration_since_unix_epoch = sys_time.duration_since(std::time::UNIX_EPOCH)?;

    let millis = duration_since_unix_epoch.as_millis() as i64;
    Ok(Timestamp::from_milliseconds(millis))
}

pub(crate) fn timestamp_to_httpdate(
    timestamp: Timestamp
) -> String {
    let millis = timestamp.milliseconds();
    let duration_since_unix_epoch = std::time::Duration::from_millis(millis as u64);
    let sys_time = std::time::UNIX_EPOCH + duration_since_unix_epoch;

    httpdate::fmt_http_date(sys_time)
}

pub(crate) fn parse_http_version(s: &str) -> Result<Version, InvalidVersion> {
    match s {
        "HTTP/0.9" => Ok(Version::HTTP_09),
        "HTTP/1.0" => Ok(Version::HTTP_10),
        "HTTP/1.1" => Ok(Version::HTTP_11),
        "HTTP/2.0" => Ok(Version::HTTP_2),
        "HTTP/3.0" => Ok(Version::HTTP_3),
        _ => Err(InvalidVersion),
    }
}