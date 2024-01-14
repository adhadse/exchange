use rocket::response::status;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;
use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, i32};

#[macro_use]
extern crate rocket;

struct User {
    id: String,
    balances: HashMap<String, f32>, // mapping between TICKER and quantity
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
struct Order {
    user_id: String,
    price: f32,
    quantity: u32,
}

#[derive(Debug, Deserialize, Clone, Copy)]
enum OrderType {
    Bid,
    Ask,
}

#[derive(Deserialize, Debug)]
struct OrderFromClient<'r> {
    user_id: &'r str,
    price: f32,
    quantity: u32,
    side: OrderType,
}

#[derive(Serialize, Debug)]
struct OrderResponse {
    filled_quantity: u32,
}

const TICKER: &str = "GOOGLE";

#[get("/<name>/<age>")]
fn hello(name: &str, age: u8) -> Json<&str> {
    Json("OK")
}

#[post("/", format = "application/json", data = "<req>")]
fn place_limit_order(app_state: &State<AppState>, req: Json<OrderFromClient<'_>>) -> Json<OrderResponse> {
    let user_id = req.user_id;
    let price = req.price;
    let quantity = req.quantity;
    let side = req.side;

    let remaining_quantity = fill_orders(app_state, side, price, quantity, user_id);

    if remaining_quantity == 0 {
        return Json(OrderResponse {
            filled_quantity: quantity,
        });
    }
    match sde {
        OrderType::Bid => {
            app_state.order_table.lock().unwrap().bids.push(Order {
                user_id,
                price,
                quantity: remaining_quantity,
            });
            app_state
                .order_table
                .lock()
                .unwrap()
                .bids
                .sort_by(|a, b| b.price.partial_cmp(&a.price));
        }
        OrderType::Ask => {
            app_state.order_table.lock().unwrap().asks.push(Order {
                user_id,
                price,
                quantity: remaining_quantity,
            });
            app_state
                .order_table
                .lock()
                .unwrap()
                .asks
                .sort_by(|a, b| a.price.partial_cmp(&b.price));
        }
    }

    Json(OrderResponse {
        filled_quantity: quantity - remaining_quantity,
    })
}

fn fill_orders(
    app_state: &State<AppState>,
    side: OrderType,
    price: f32,
    quantity: u32,
    user_id: &str,
) -> u32 {
    let mut remaining_quantity = quantity;
    match side {
        OrderType::Bid => {
            for ask in app_state.order_table.lock().unwrap().asks.iter_mut().rev() {
                if ask.price > price {
                    // if this bid isn't satisfied with lowest ask
                    // nothing else would match further than that.
                    break;
                }
                // Buy it at ask's price, which is lower than what Bid offered
                if ask.quantity > remaining_quantity {
                    // bid gets fullfilled; ask still remains on ordertable with reduced quantity
                    ask.quantity -= remaining_quantity;
                    flip_balance(
                        &app_state,
                        &ask.user_id,
                        user_id,
                        remaining_quantity,
                        ask.price,
                    );
                    return 0;
                } else {
                    // bids get half fullfilled; ask is dropped from order table
                    remaining_quantity -= ask.quantity;
                    flip_balance(&app_state, &ask.user_id, &user_id, ask.quantity, ask.price);
                    app_state.order_table.lock().unwrap().asks.pop(); // it must be last because of guard condition on top
                }
            }
        }
        OrderType::Ask => {
            for bid in app_state.order_table.lock().unwrap().bids.iter_mut().rev() {
                if bid.price < price {
                    // if this ask isn't satisfied with lowest bid
                    // nothing else would match further than that
                    break;
                }
                // Sell it at Ask's price, which is lower than what Bid offered
                if bid.quantity > remaining_quantity {
                    // ask gets fullfilled, bid still remains on ordertable with reduced quantity
                    bid.quantity -= remaining_quantity;
                    flip_balance(
                        &app_state,
                        &bid.user_id,
                        &user_id,
                        remaining_quantity,
                        price,
                    );
                    return 0;
                } else {
                    remaining_quantity -= bid.quantity;
                    flip_balance(&app_state, &bid.user_id, &user_id, bid.quantity, price);
                    app_state.order_table.lock().unwrap().bids.pop();
                }
            }
        }
    }
    remaining_quantity
}

fn flip_balance(
    app_state: &State<AppState>,
    user_id1: &str,
    user_id2: &str,
    quantity: u32,
    price: f32,
) {
    let binding = app_state.users.lock().unwrap();

    let user_1 = binding.iter().filter(|user| user.id == user_id1).next();
    let user_2 = binding.iter().filter(|user| user.id == user_id2).next();

    if user_1.is_none() || user_2.is_none() {
        return;
    }

    // change stock quantity
    *user_1.unwrap().balances.get_mut(TICKER).unwrap() -= quantity as f32;
    *user_2.unwrap().balances.get_mut(TICKER).unwrap() += quantity as f32;
    // change fund quantity
    *user_1.unwrap().balances.get_mut("USD").unwrap() += (quantity as f32) * price;
    *user_2.unwrap().balances.get_mut("USD").unwrap() -= (quantity as f32) * price;
}

struct OrderTable {
    bids: Vec<Order>, // bids is stored in increasing order [1, 2, 3]
    asks: Vec<Order>, // asks is stored in decreasing order [3, 2, 1]
}

struct AppState {
    order_table: Mutex<OrderTable>,
    users: Mutex<Vec<User>>,
}

#[launch]
fn rocket() -> _ {
    //    let users: Vec<User> = ;

    rocket::build()
        .mount("/hello", routes![hello])
        .mount("/order", routes![place_limit_order])
        .manage(AppState {
            order_table: Mutex::new(OrderTable {
                bids: Vec::new(),
                asks: Vec::new(),
            }),
            users: Mutex::new(Vec::from([
                User {
                    id: "1".to_string(),
                    balances: HashMap::from([
                        ("GOOGLE".to_string(), 10.0),
                        ("USD".to_string(), 50_000.0),
                    ]),
                },
                User {
                    id: "2".to_string(),
                    balances: HashMap::from([
                        ("GOOGLE".to_string(), 10.0),
                        ("USD".to_string(), 50_000.0),
                    ]),
                },
            ])),
        })
}
