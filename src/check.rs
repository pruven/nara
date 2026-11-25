use std::time::{Duration, Instant};

use url::Url;

pub fn parse(input: &str) -> Result<Url, &'static str> {
    let url = Url::parse(input.trim()).map_err(|_| "Enter a complete HTTP or HTTPS link.")?;
    if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
        return Err("Nara handles HTTP and HTTPS links.");
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Links containing credentials are unsupported.");
    }
    Ok(url)
}

pub struct Check {
    pub status: String,
    pub elapsed: Duration,
    pub destination: Url,
    pub redirects: usize,
}

pub fn check(url: Url) -> Result<Check, String> {
    follow_redirects(url, ureq::http::Method::HEAD)
}

pub fn remove_tracking(url: &Url) -> Option<(Url, usize)> {
    let query = url.query()?;
    let mut fields = Vec::new();
    let mut removed = 0;
    for field in query.split('&') {
        let name = url::form_urlencoded::parse(field.as_bytes())
            .next()
            .map(|(name, _)| name.to_ascii_lowercase())
            .unwrap_or_default();
        if matches!(
            name.as_str(),
            "sig" | "signature" | "x-amz-signature" | "x-goog-signature"
        ) {
            return None;
        }
        if name.starts_with("utm_")
            || matches!(
                name.as_str(),
                "fbclid"
                    | "gclid"
                    | "dclid"
                    | "msclkid"
                    | "ttclid"
                    | "twclid"
                    | "mc_cid"
                    | "mc_eid"
            )
        {
            removed += 1;
        } else {
            fields.push(field);
        }
    }
    if removed == 0 {
        return None;
    }
    let mut cleaned = url.clone();
    let query = fields.join("&");
    cleaned.set_query((!query.is_empty()).then_some(query.as_str()));
    Some((cleaned, removed))
}

pub fn unshorten(url: Url) -> Result<(Url, usize), String> {
    follow_redirects(url, ureq::http::Method::GET)
        .map(|result| (result.destination, result.redirects))
}

fn follow_redirects(mut url: Url, method: ureq::http::Method) -> Result<Check, String> {
    let start = Instant::now();
    let limit = Duration::from_secs(8);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into();

    for redirects in 0..=5 {
        let remaining = limit.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            return Err("Resolving the link timed out.".to_owned());
        }
        let mut request_url = url.clone();
        request_url.set_fragment(None);
        let request = if method == ureq::http::Method::HEAD {
            agent.head(request_url.as_str())
        } else {
            agent.get(request_url.as_str())
        };
        let response = request
            .config()
            .timeout_global(Some(remaining))
            .build()
            .call()
            .map_err(|error| match error {
                ureq::Error::Timeout(_) => "The link request timed out.".to_owned(),
                _ => format!("Could not complete the link request. {error}"),
            })?;
        let location = response.headers().get("location");
        if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) || location.is_none()
        {
            return Ok(Check {
                status: response.status().to_string(),
                elapsed: start.elapsed(),
                destination: url,
                redirects,
            });
        }
        let location = location
            .and_then(|value| value.to_str().ok())
            .ok_or("The redirect contains an invalid address.")?;
        let mut next = url
            .join(location)
            .map_err(|_| "The redirect contains an invalid address.")?;
        parse(next.as_str()).map_err(str::to_owned)?;
        if next.fragment().is_none() {
            next.set_fragment(url.fragment());
        }
        url = next;
    }
    Err("The link exceeded the five-redirect limit.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    #[test]
    fn accepts_web_links_without_embedded_credentials() {
        assert!(parse(" https://example.org/path#section ").is_ok());
        assert!(parse("file:///tmp/test").is_err());
        assert!(parse("https://example.org@other.example/").is_err());
    }

    #[test]
    fn removes_tracking_without_reencoding_other_parameters() {
        let original =
            parse("https://example.org/p?utm_source=test&q=a%20b&q=%2B&FBCLID=123#section")
                .unwrap();
        let (cleaned, count) = remove_tracking(&original).unwrap();
        assert_eq!(count, 2);
        assert_eq!(
            cleaned.as_str(),
            "https://example.org/p?q=a%20b&q=%2B#section"
        );
        assert!(
            remove_tracking(
                &parse("https://example.org/?utm_medium=x&X-Amz-Signature=abc").unwrap()
            )
            .is_none()
        );
    }

    #[test]
    fn reports_redirect_target_and_http_error_status() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            for (path, response) in [
                ("/start", "302 Found\r\nLocation: /end#target"),
                ("/end", "403 Forbidden"),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut received = Vec::new();
                while !received.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    received.push(byte[0]);
                }
                assert!(received.starts_with(format!("HEAD {path} ").as_bytes()));
                write!(
                    stream,
                    "HTTP/1.1 {response}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
            }
        });
        let result = check(parse(&format!("{origin}/start#fragment")).unwrap()).unwrap();
        assert_eq!(result.status, "403 Forbidden");
        assert_eq!(result.destination.as_str(), format!("{origin}/end#target"));
        assert_eq!(result.redirects, 1);
        server.join().unwrap();
    }
}
