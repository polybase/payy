use super::{State, blocks, element, health, height, merkle, smirk, stats, txn};
use actix_web::web;

pub fn configure_routes(state: State) -> Box<dyn FnOnce(&mut web::ServiceConfig)> {
    Box::new(move |cfg: &mut web::ServiceConfig| {
        cfg.app_data(web::Data::new(state))
            .service(web::resource("/health").get(health::get_health))
            .service(web::resource("/height").get(height::get_height))
            .service(web::resource("/merkle").get(merkle::get_merkle_paths))
            .service(web::resource("/elements/{element}").get(element::get_element))
            .service(
                web::resource("/elements")
                    .get(element::list_elements)
                    .post(element::list_elements_post),
            )
            .service(web::resource("/blocks/{height}/tree").get(blocks::get_block_tree))
            .service(web::resource("/blocks/{block}").get(blocks::get_block))
            .service(web::resource("/blocks").get(blocks::list_blocks))
            .service(web::resource("/transaction").post(txn::submit_txn))
            .service(web::resource("/transactions/{hash}").get(txn::get_txn))
            .service(
                web::resource("/transactions")
                    .get(txn::list_txns)
                    .post(txn::submit_txn),
            )
            .service(web::resource("/stats").get(stats::get_stats))
            .service(web::resource("/smirk/elements/all").get(smirk::get_all_smirk_elements));
    })
}
