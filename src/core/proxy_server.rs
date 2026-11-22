use std::sync::Arc;
use tokio::net::TcpListener;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use crate::core::engine::EngineDispatcher;
use crate::core::types::ConnectionRequest;
use crate::core::executor::{build_candidates, Racer};
use futures_util::stream::iter;

use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use http_body_util::combinators::BoxBody;

type RespBody = BoxBody<Bytes, hyper::Error>;

fn empty_body() -> RespBody {
    Empty::<Bytes>::new().map_err(|never| match never {}).boxed()
}

pub async fn start_proxy_server(dispatcher: Arc<EngineDispatcher>) -> anyhow::Result<()> {
    // 这里如果报错，会向上抛出，由 background.rs 捕获
    let listener = TcpListener::bind("127.0.0.1:8443").await?;
    tracing::info!("代理服务启动成功: 127.0.0.1:8443");

    loop {
        let (stream, _) = listener.accept().await?;
        let dispatcher = dispatcher.clone();

        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| {
                handle_connect(req, dispatcher.clone())
            });

            if let Err(err) = http1::Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                tracing::debug!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn handle_connect(
    req: Request<hyper::body::Incoming>,
    dispatcher: Arc<EngineDispatcher>,
) -> Result<Response<RespBody>, hyper::Error> {
    if req.method() != hyper::Method::CONNECT {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(empty_body())
            .unwrap());
    }

    let host = req.uri().host().unwrap_or_default().to_string();
    let port = req.uri().port_u16().unwrap_or(443);
    
    let conn_req = ConnectionRequest {
        target_addr: "0.0.0.0:0".parse().unwrap(),
        domain: Some(host.clone()),
        port,
    };

    let plan = dispatcher.dispatch(&conn_req).await.unwrap_or_else(|_| {
        crate::core::types::ExecutionPlan {
            candidates: vec![crate::core::types::RouteAttempt::Direct],
            fallback: false,
        }
    });

    tracing::info!("Routing {}:{} -> {:?}", host, port, plan.candidates);

    tokio::spawn(async move {
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                let mut upgraded = TokioIo::new(upgraded);
                
                let candidates = build_candidates(plan, conn_req.clone());
                let stream_iter = iter(candidates);
                
                match Racer::race(stream_iter).await {
                    Ok(mut upstream) => {
                        tracing::info!("Tunnel established: {}:{}", host, port);
                        if let Err(e) = tokio::io::copy_bidirectional(&mut upgraded, &mut upstream).await {
                            tracing::debug!("Copy disconnected: {}:{} - {}", host, port, e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to connect {}:{} - {}", host, port, e);
                    }
                }
            }
            Err(e) => tracing::error!("Upgrade error: {}", e),
        }
    });

    Ok(Response::new(empty_body()))
}