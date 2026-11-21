use std::pin::Pin;
use std::future::Future;
use anyhow::{anyhow, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use crate::core::executor::RaceCandidate;
use crate::core::types::{BoxedStream, ConnectionRequest};

#[derive(Clone)]
pub struct ProxyCandidate {
    pub req: ConnectionRequest,
    pub node_id: String,
}

impl RaceCandidate for ProxyCandidate {
    fn label(&self) -> String { format!("Proxy({})", self.node_id) }

    fn connect(self) -> Pin<Box<dyn Future<Output = Result<(String, BoxedStream)>> + Send>> {
        Box::pin(async move {
            let label = self.label();
            
            // 从环境变量获取代理地址，默认连接本地的 Clash/sing-box 端口
            let proxy_url = std::env::var("PROXY_NODE_URL").unwrap_or_else(|_| "socks5://127.0.0.1:7890".to_string());
            let proxy_addr = proxy_url.trim_start_matches("socks5://");
            
            let mut stream = TcpStream::connect(proxy_addr).await
                .map_err(|e| anyhow!("Failed to connect to SOCKS5 proxy {}: {}", proxy_addr, e))?;

            // 1. SOCKS5 握手：发送 [版本号 5, 支持的认证方法数 1, 无认证 0]
            stream.write_all(&[0x05, 0x01, 0x00]).await?;
            let mut resp = [0u8; 2];
            stream.read_exact(&mut resp).await?;
            if resp != [0x05, 0x00] {
                return Err(anyhow!("SOCKS5 auth failed: {:?}", resp));
            }

            // 2. 发起 CONNECT 请求：请求连接目标域名和端口
            let domain = self.req.domain.unwrap_or_default();
            let port = self.req.port;
            let mut connect_req = vec![
                0x05, // Version
                0x01, // Command: CONNECT
                0x00, // Reserved
                0x03, // Address type: Domain name
                domain.len() as u8,
            ];
            connect_req.extend_from_slice(domain.as_bytes());
            connect_req.extend_from_slice(&port.to_be_bytes());

            stream.write_all(&connect_req).await?;

            // 3. 读取代理响应
            let mut reply = [0u8; 4];
            stream.read_exact(&mut reply).await?;
            if reply[1] != 0x00 {
                return Err(anyhow!("SOCKS5 connect failed: error code {}", reply[1]));
            }
            
            // 4. 读取并丢弃 BND.ADDR 和 BND.PORT，将流指针移到正确位置
            match reply[3] {
                0x01 => { // IPv4: 4 bytes IP + 2 bytes port
                    let mut addr = [0u8; 4 + 2];
                    stream.read_exact(&mut addr).await?;
                }
                0x03 => { // Domain: 1 byte length + N bytes domain + 2 bytes port
                    let mut len = [0u8; 1];
                    stream.read_exact(&mut len).await?;
                    let mut addr = vec![0u8; len[0] as usize + 2];
                    stream.read_exact(&mut addr).await?;
                }
                0x04 => { // IPv6: 16 bytes IP + 2 bytes port
                    let mut addr = [0u8; 16 + 2];
                    stream.read_exact(&mut addr).await?;
                }
                _ => return Err(anyhow!("Unknown ATYP in SOCKS5 response")),
            }

            Ok((label, Box::new(stream) as BoxedStream))
        })
    }
}