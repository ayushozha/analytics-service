use std::net::IpAddr;

use maxminddb::Reader;

#[derive(Debug, Clone, Default)]
pub struct GeoResult {
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
}

pub fn lookup_ip(reader: &Reader<Vec<u8>>, ip_str: &str) -> GeoResult {
    let ip: IpAddr = match ip_str.parse() {
        Ok(ip) => ip,
        Err(_) => return GeoResult::default(),
    };

    match reader.lookup::<maxminddb::geoip2::City>(ip) {
        Ok(city) => GeoResult {
            country: city
                .country
                .and_then(|c| c.iso_code)
                .map(|s| s.to_string()),
            region: city
                .subdivisions
                .and_then(|s| s.first().cloned())
                .and_then(|s| s.names)
                .and_then(|n| n.get("en").copied())
                .map(|s| s.to_string()),
            city: city
                .city
                .and_then(|c| c.names)
                .and_then(|n| n.get("en").copied())
                .map(|s| s.to_string()),
        },
        Err(_) => GeoResult::default(),
    }
}
