use fe2o3_amqp_types::primitives::Timestamp;

use crate::error::ProjectedModeError;

pub(crate) fn httpdate_to_timestamp(
    httpdate: &str
) -> Result<Timestamp, ProjectedModeError> {
    let sys_time = httpdate::parse_http_date(httpdate)?;
    let duration_since_unix_epoch = sys_time.duration_since(std::time::UNIX_EPOCH)?;

    let millis = duration_since_unix_epoch.as_millis() as i64;
    Ok(Timestamp::from_milliseconds(millis))
}