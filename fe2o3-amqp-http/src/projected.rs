use fe2o3_amqp_types::{messaging::{header, map_builder::MapBuilder, properties, ApplicationProperties, Data, Message, Properties}, primitives::{Binary, SimpleValue, Value}};
use http::{header::{ToStrError, CONTENT_ENCODING, CONTENT_TYPE, DATE, EXPIRES, FROM}, Request, Response};

use crate::error::ProjectedModeError;

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

fn project_headers<BE>(
    mut prop_builder: PropertiesBuilder,
    mut app_prop_builder: ApplicationPropertiesBuilder,
    headers: &http::HeaderMap
) -> Result<(Properties, ApplicationProperties), ProjectedModeError<BE>> {
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
        "te", "trailer", "transfer-encoding", "content-length", "via", "connection", "upgrade"
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
        "content-type", "content-encoding", "date", "from", "expires"
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
    let iter = headers.iter()
        .filter(|(name, _)| !EXCLUDED_HEADERS_LOWERCASE.contains(&name.as_str().to_lowercase().as_str())) // TODO: how to avoid creating new strings?
        .filter(|(name, _)| !SPECIAL_HEADERS_LOWERCASE.contains(&name.as_str().to_lowercase().as_str()));
    for (name, value) in iter {
        app_prop_builder = app_prop_builder.insert(name.as_str().to_lowercase(), value.to_str()?);
    }
    
    Ok((prop_builder.build(), app_prop_builder.build()))
}

impl<B> TryIntoProjected<B> for Request<B> 
where 
    B: TryInto<Data>,
    B::Error: std::error::Error,
{
    type Error = ProjectedModeError<B::Error>;

    /// Implement HTTP mapping to AMQP message in projected mode.
    /// 
    /// See Section 4.1 for more details.
    fn into_projected(self) -> Result<Message<Data>, Self::Error> {
        //  An implementation MUST ignore and exclude all RFC7230 headers and
        //  RFC7230 information items not explicitly covered below

        // MUST include all headers and information items from RFC7231 and other 
        // HTTP extension specifications.
        
        let (parts, body) = self.into_parts();

        // 4.1.1 Request Line
        let prop_builder = Properties::builder()
            // HTTP method MUST be set in the properties section, subject field. 
            .subject(parts.method.as_str())
            // HTTP request-target value MUST be URI-decoded and set in the
            // properties section, to field.
            .to(parts.uri.to_string());
        let app_prop_builder = ApplicationProperties::builder()
            // The version value SHOULD be set in application-properties section, as
            // value of the “http:request” string property. The assumed default
            // value is “1.1” if the property is absent. 
            .insert("http:request", format!("{:?}", parts.version)); // TODO: something better than Debug fmt?

        let (properties, application_properties) = project_headers(prop_builder, app_prop_builder, &parts.headers)?;

        let data = body.try_into().map_err(ProjectedModeError::Body)?;
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
    type Error = ProjectedModeError<B::Error>;

    fn into_projected(self) -> Result<Message<Data>, Self::Error> {
        let (parts, body) = self.into_parts();

        // 4.1.2 Status Line
        let app_prop_builder = ApplicationProperties::builder()
            // The version value SHOULD be set in application-properties section, as value of the 
            // “http:response” string property. The assumed default value is “1.1” if the property is absent.
            .insert("http:response", format!("{:?}", parts.version)); // TODO: something better than Debug fmt?
        let prop_builder = Properties::builder()
            // HTTP status code MUST be set in the properties section, subject field.
            .subject(parts.status.as_str());
            // HTTP reason phrase is OPTIONAL and is omitted here.

        let (properties, application_properties) = project_headers(prop_builder, app_prop_builder, &parts.headers)?;

        let data = body.try_into().map_err(ProjectedModeError::Body)?;
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

    type Error = ProjectedModeError<B::Error>;

    fn try_from_projected(msg: Message<Data>) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl<B> TryFromProjected for Response<B> 
where 
    B: TryFrom<Data>,
    B::Error: std::error::Error,
{
    type Body = B;

    type Error = ProjectedModeError<B::Error>;

    fn try_from_projected(msg: Message<Data>) -> Result<Self, Self::Error> {
        todo!()
    }
}