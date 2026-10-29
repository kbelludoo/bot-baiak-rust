use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::Duration;
use tracing::warn;

pub fn is_ip_available(ip_str: &str) -> bool {
    if let Ok(ip) = ip_str.parse::<IpAddr>() {
        // Tenta fazer um bind temporário para conferir se o SO possui essa interface de rede
        let bind_addr = SocketAddr::new(ip, 0);
        std::net::TcpListener::bind(bind_addr).is_ok()
    } else {
        false
    }
}

pub fn build_http_client(bind_ip: &str, timeout_secs: u64) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs));

    if !bind_ip.is_empty() {
        if is_ip_available(bind_ip) {
            if let Ok(ip) = bind_ip.parse::<IpAddr>() {
                builder = builder.local_address(ip);
            }
        } else {
            warn!("⚠️ [Network] IP de bind '{}' não está ativo nesta máquina. Usando rota padrão do SO.", bind_ip);
        }
    }

    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}
