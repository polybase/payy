#![warn(clippy::unwrap_used, clippy::expect_used)]

use super::routes::error::Error as RoutesError;
use crate::{NodeShared, TxnStats};
use actix_cors::Cors;
use actix_server::Server;
use actix_web::{App, HttpResponse, HttpServer, Responder, dev::Service, web};
use rpc::{
    error::{HTTPError, not_found_error_handler},
    middleware::Middleware,
};
use std::sync::Arc;

use super::routes::{State, configure_routes};

async fn root() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(format!(
            "{{ \"network\": \"polybase\", \"service\": \"node\", \"version\": \"{}\" }}",
            env!("CARGO_PKG_VERSION")
        ))
}

#[tracing::instrument(err, skip(node))]
pub fn create_rpc_server(
    rpc_laddr: &str,
    health_check_commit_interval_sec: u64,
    node: Arc<NodeShared>,
    txn_stats: Arc<TxnStats>,
) -> Result<Server, std::io::Error> {
    Ok(HttpServer::new(move || {
        let cors: Cors = Cors::permissive();

        let state = State {
            node: Arc::clone(&node),
            health_check_commit_interval_sec,
            txn_stats: Arc::clone(&txn_stats),
        };

        App::new()
            .wrap(cors)
            .wrap(Middleware)
            .wrap_fn({
                let node = Arc::clone(&node);

                move |req, srv| {
                    let fut = srv.call(req);
                    let is_out_of_sync = node.is_out_of_sync();

                    async move {
                        if is_out_of_sync {
                            let err = HTTPError::from(RoutesError::OutOfSync);
                            Err(err.into())
                        } else {
                            fut.await
                        }
                    }
                }
            })
            .service(web::resource("/").get(root))
            .service(web::scope("/v0").configure(configure_routes(state)))
            .default_service(web::route().to(not_found_error_handler))
    })
    .bind(rpc_laddr)? // todo - better error handling
    .run())
}
