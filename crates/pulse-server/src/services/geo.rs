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

    match reader.lookup(ip) {
        Ok(result) => GeoResult {
            country: result
                .decode_path::<String>(&maxminddb::path!["country", "iso_code"])
                .ok()
                .flatten(),
            region: result
                .decode_path::<String>(&maxminddb::path!["subdivisions", 0, "names", "en"])
                .ok()
                .flatten(),
            city: result
                .decode_path::<String>(&maxminddb::path!["city", "names", "en"])
                .ok()
                .flatten(),
        },
        Err(_) => GeoResult::default(),
    }
}
