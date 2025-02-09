use std::str::FromStr;

use fe2o3_amqp_types::{
    messaging::{
        header, map_builder::MapBuilder, properties, ApplicationProperties, Data, Message,
        Properties,
    },
    primitives::{Binary, SimpleValue, Value},
};
use http::{
    header::{ToStrError, CONTENT_ENCODING, CONTENT_TYPE, DATE, EXPIRES, FROM}, HeaderName, HeaderValue, Method, Request, Response
};

use crate::{error::{TryFromProjectedError, TryIntoProjectedError}, util::{parse_http_version, timestamp_to_httpdate}};

type PropertiesBuilder = properties::Builder;
type ApplicationPropertiesBuilder = MapBuilder<String, SimpleValue, ApplicationProperties>;

/// Type Parameter B is the body type of the HTTP message.
///
/// > When interpreting the message content, it MUST be considered as
/// equivalent to a single data section obtained by concatenating all the
/// data sections, the data section boundaries MUST be ignored.
pub trait TryIntoProjected<B>
where
    B: TryInto<Data>,
    B::Error: std::error::Error,
{
    type Error: std::error::Error;

    fn into_projected(self) -> Result<Message<Data>, Self::Error>;
}

fn project_request_line(
    prop_builder: PropertiesBuilder,
    app_prop_builder: ApplicationPropertiesBuilder,
    method: &str,
    uri: &str,
    version: &http::Version,
) -> (PropertiesBuilder, ApplicationPropertiesBuilder) {
    // 4.1.1 Request Line
    let prop_builder = prop_builder.subject(method).to(uri);
    let app_prop_builder = app_prop_builder.insert("http:request", format!("{:?}", version)); // TODO: something better than Debug fmt?

    (prop_builder, app_prop_builder)
}

fn project_status_line(
    prop_builder: PropertiesBuilder,
    app_prop_builder: ApplicationPropertiesBuilder,
    status: &http::StatusCode,
    version: &http::Version,
) -> (PropertiesBuilder, ApplicationPropertiesBuilder) {
    // 4.1.2 Status Line
    let app_prop_builder = app_prop_builder.insert("http:response", format!("{:?}", version)); // TODO: something better than Debug fmt?
    let prop_builder = prop_builder.subject(status.as_str());

    (prop_builder, app_prop_builder)
}

fn project_headers<BE>(
    mut prop_builder: PropertiesBuilder,
    mut app_prop_builder: ApplicationPropertiesBuilder,
    headers: &http::HeaderMap,
) -> Result<(PropertiesBuilder, ApplicationPropertiesBuilder), TryIntoProjectedError<BE>> {
    // 4.1.3 Headers
    // The following HTTP Headers defined in RFC7230 MUST NOT be mapped into AMQP HTTP messages
    // - TE
    // - Trailer
    // - Transfer-Encoding
    // - Content-Length
    // - Via
    // - Connection
    // - Upgrade
    const EXCLUDED_HEADERS_LOWERCASE: [&str; 7] = [
        "te",
        "trailer",
        "transfer-encoding",
        "content-length",
        "via",
        "connection",
        "upgrade",
    ];

    // The HTTP Host information MUST follow the addressing rules defined in section 3. While the Host header
    // is required in RFC7230, it is OPTIONAL in HTTP AMQP because the container is already uniquely
    // identified through other means. The Host value is set in the application-properties section, as value of the
    // “http:host” string property. When the property is omitted, the default value is a container-defined scope
    // identifier.

    // The following RFC7231 headers have special mappings:
    // - Content-Type maps to the properties section, content-type field.
    if let Some(content_type) = headers.get(CONTENT_TYPE) {
        prop_builder = prop_builder.content_type(content_type.to_str()?);
    }
    // - Content-Encoding maps to the properties section, content-encoding field.
    if let Some(content_encoding) = headers.get(CONTENT_ENCODING) {
        prop_builder = prop_builder.content_encoding(content_encoding.to_str()?);
    }
    // - Date maps to the properties section, creation-time field.
    if let Some(date) = headers.get(DATE) {
        let date_str = date.to_str()?;
        let timestamp = crate::util::httpdate_to_timestamp(date_str)?;
        prop_builder = prop_builder.creation_time(timestamp);
    }
    // - From maps to the properties section, user-id field.
    if let Some(from) = headers.get(FROM) {
        prop_builder = prop_builder.user_id(Binary::from(from.as_bytes()));
    }

    // The following RFC7234 header has a special mapping
    // - The Expires value maps to the properties section, absolute-expiry-time field.
    if let Some(expires) = headers.get(EXPIRES) {
        let expires_str = expires.to_str()?;
        let timestamp = crate::util::httpdate_to_timestamp(expires_str)?;
        prop_builder = prop_builder.absolute_expiry_time(timestamp);
    }

    const SPECIAL_HEADERS_LOWERCASE: [&str; 5] = [
        "content-type",
        "content-encoding",
        "date",
        "from",
        "expires",
    ];

    // All other HTTP headers, including those defined in RFC7231 and any
    // headers defined by formal HTTP extensions as well as any application
    // specific HTTP headers are added to the application-properties section
    // of the message. The header names are not prefixed. Headers with
    // special mappings MUST NOT be added to the application-properties
    // section
    //
    // Because HTTP header names are case-insensitive but AMQP property names are case-sensitive, all
    // HTTP header names MUST be converted to lower case as they are mapped to AMQP application
    // property names. The type of all mapped header values is string.
    let iter = headers
        .iter()
        .filter(|(name, _)| {
            !EXCLUDED_HEADERS_LOWERCASE.contains(&name.as_str().to_lowercase().as_str())
        }) // TODO: how to avoid creating new strings?
        .filter(|(name, _)| {
            !SPECIAL_HEADERS_LOWERCASE.contains(&name.as_str().to_lowercase().as_str())
        });
    for (name, value) in iter {
        app_prop_builder = app_prop_builder.insert(name.as_str().to_lowercase(), value.to_str()?);
    }

    Ok((prop_builder, app_prop_builder))
}

impl<B> TryIntoProjected<B> for Request<B>
where
    B: TryInto<Data>,
    B::Error: std::error::Error,
{
    type Error = TryIntoProjectedError<B::Error>;

    /// Implement HTTP mapping to AMQP message in projected mode.
    ///
    /// See Section 4.1 for more details.
    fn into_projected(self) -> Result<Message<Data>, Self::Error> {
        //  An implementation MUST ignore and exclude all RFC7230 headers and
        //  RFC7230 information items not explicitly covered below

        // MUST include all headers and information items from RFC7231 and other
        // HTTP extension specifications.

        let (parts, body) = self.into_parts();

        let prop_builder = Properties::builder();
        let app_prop_builder = ApplicationProperties::builder();

        let (prop_builder, app_prop_builder) = project_request_line(
            prop_builder,
            app_prop_builder,
            parts.method.as_str(),
            parts.uri.to_string().as_str(),
            &parts.version,
        );
        let (prop_builder, app_prop_builder) =
            project_headers(prop_builder, app_prop_builder, &parts.headers)?;

        let properties = prop_builder.build();
        let application_properties = app_prop_builder.build();

        let data = body.try_into().map_err(TryIntoProjectedError::Body)?;
        let msg = Message::builder()
            .properties(properties)
            .application_properties(application_properties)
            .data(data)
            .build();

        Ok(msg)
    }
}

impl<B> TryIntoProjected<B> for Response<B>
where
    B: TryInto<Data>,
    B::Error: std::error::Error,
{
    type Error = TryIntoProjectedError<B::Error>;

    fn into_projected(self) -> Result<Message<Data>, Self::Error> {
        let (parts, body) = self.into_parts();

        let prop_builder = Properties::builder();
        let app_prop_builder = ApplicationProperties::builder();

        let (prop_builder, app_prop_builder) = project_status_line(
            prop_builder,
            app_prop_builder,
            &parts.status,
            &parts.version,
        );
        let (prop_builder, app_prop_builder) =
            project_headers(prop_builder, app_prop_builder, &parts.headers)?;

        let properties = prop_builder.build();
        let application_properties = app_prop_builder.build();

        let data = body.try_into().map_err(TryIntoProjectedError::Body)?;
        let msg = Message::builder()
            .properties(properties)
            .application_properties(application_properties)
            .data(data)
            .build();

        Ok(msg)
    }
}

pub trait TryFromProjected
where
    Self: Sized,
{
    type Body;
    type Error: std::error::Error;

    fn try_from_projected(msg: Message<Data>) -> Result<Self, Self::Error>;
}

impl<B> TryFromProjected for Request<B>
where
    B: TryFrom<Data>,
    B::Error: std::error::Error,
{
    type Body = B;

    type Error = TryFromProjectedError<B::Error>;

    fn try_from_projected(msg: Message<Data>) -> Result<Self, Self::Error> {
        // If properties are missing, method is definitely missing
        let properties = msg.properties.ok_or(TryFromProjectedError::MethodNotPresent)?;
        // There shouldn't be any allocation for the default 
        let application_properties = msg.application_properties.unwrap_or_default();
        
        // Get request line
        let method = properties
            .subject
            .map(|s| Method::from_str(&s))
            .ok_or(TryFromProjectedError::MethodNotPresent)??;
        let uri = properties
            .to
            .ok_or(TryFromProjectedError::RequestTargetNotPresent)?;
        let version = application_properties
            .get("http:request")
            .map(|v| match v {
                SimpleValue::String(s) => Ok(s.as_str()),
                _ => Err(TryFromProjectedError::VersionNotString), 
            })
            .unwrap_or_else(|| Ok("1.1"))?; // default to 1.1

        
        let version = parse_http_version(version)?;
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .version(version);

        // Get special headers
        if let Some(content_type) = properties.content_type {
            builder = builder.header(CONTENT_TYPE, HeaderValue::from_str(&content_type.0)?);
        }
        if let Some(content_encoding) = properties.content_encoding {
            builder = builder.header(CONTENT_ENCODING, HeaderValue::from_str(&content_encoding.0)?);
        }
        if let Some(creation_time) = properties.creation_time {
            let date_str = timestamp_to_httpdate(creation_time);
            builder = builder.header(DATE, HeaderValue::from_str(&date_str)?);
        }
        if let Some(user_id) = properties.user_id {
            builder = builder.header(FROM, HeaderValue::from_bytes(&user_id)?);
        }
        if let Some(absolute_expiry_time) = properties.absolute_expiry_time {
            let expires_str = timestamp_to_httpdate(absolute_expiry_time);
            builder = builder.header(EXPIRES, HeaderValue::from_str(&expires_str)?);
        }

        const HEADERS_TO_IGNORE_LOWERCASE: [&str; 1] = [
            "http:request",
        ];

        // Get other headers
        // Compare lowercased header names to SPECIAL_HEADERS_LOWERCASE
        let iter = application_properties.iter().filter_map(|(name, value)| {
            let name = name.to_lowercase();
            if HEADERS_TO_IGNORE_LOWERCASE.contains(&name.as_str()) {
                None
            } else {
                // TODO: return an error or ignore the wrong header?
                let name = HeaderName::from_lowercase(name.as_bytes()).ok()?;
                let value = match value {
                    SimpleValue::String(s) => HeaderValue::from_str(s).ok()?,
                    _ => return None,
                };
                Some((name, value))
            }
        });

        for (name, value) in iter {
            builder = builder.header(name, value);
        }
        
        let body = B::try_from(msg.body).map_err(TryFromProjectedError::Body)?;
        builder.body(body).map_err(Into::into)
    }
}

impl<B> TryFromProjected for Response<B>
where
    B: TryFrom<Data>,
    B::Error: std::error::Error,
{
    type Body = B;

    type Error = TryFromProjectedError<B::Error>;

    fn try_from_projected(msg: Message<Data>) -> Result<Self, Self::Error> {
        let properties = msg.properties.ok_or(TryFromProjectedError::StatusNotPresent)?;
        let application_properties = msg.application_properties.unwrap_or_default();
        
        // Get status line
        let status = properties
            .subject
            .map(|s| http::StatusCode::from_bytes(s.as_bytes()))
            .ok_or(TryFromProjectedError::StatusNotPresent)??;
        let version = application_properties
            .get("http:response")
            .map(|v| match v {
                SimpleValue::String(s) => Ok(s.as_str()),
                _ => Err(TryFromProjectedError::VersionNotString), 
            })
            .unwrap_or_else(|| Ok("1.1"))?; // default to 1.1

        let version = parse_http_version(version)?;
        let mut builder = Response::builder()
            .status(status)
            .version(version);

        // Get special headers
        if let Some(content_type) = properties.content_type {
            builder = builder.header(CONTENT_TYPE, HeaderValue::from_str(&content_type.0)?);
        }
        if let Some(content_encoding) = properties.content_encoding {
            builder = builder.header(CONTENT_ENCODING, HeaderValue::from_str(&content_encoding.0)?);
        }
        if let Some(creation_time) = properties.creation_time {
            let date_str = timestamp_to_httpdate(creation_time);
            builder = builder.header(DATE, HeaderValue::from_str(&date_str)?);
        }
        if let Some(user_id) = properties.user_id {
            builder = builder.header(FROM, HeaderValue::from_bytes(&user_id)?);
        }
        if let Some(absolute_expiry_time) = properties.absolute_expiry_time {
            let expires_str = timestamp_to_httpdate(absolute_expiry_time);
            builder = builder.header(EXPIRES, HeaderValue::from_str(&expires_str)?);
        }

        // TODO: other headers that should be ignored?
        const HEADERS_TO_IGNORE_LOWERCASE: [&str; 1] = [
            "http:response",
        ];

        // Get other headers
        // Compare lowercased header names to SPECIAL_HEADERS_LOWERCASE
        let iter = application_properties.iter().filter_map(|(name, value)| {
            let name = name.to_lowercase();
            if HEADERS_TO_IGNORE_LOWERCASE.contains(&name.as_str()) {
                None
            } else {
                // TODO: return an error or ignore the wrong header?
                let name = HeaderName::from_lowercase(name.as_bytes()).ok()?;
                let value = match value {
                    SimpleValue::String(s) => HeaderValue::from_str(s).ok()?,
                    _ => return None,
                };
                Some((name, value))
            }
        });

        for (name, value) in iter {
            builder = builder.header(name, value);
        }

        let body = B::try_from(msg.body).map_err(TryFromProjectedError::Body)?;
        builder.body(body).map_err(Into::into)
    }
}
