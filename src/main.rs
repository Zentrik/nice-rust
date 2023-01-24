use std::env;
use std::collections::HashMap;

extern crate num_bigint;
use num_bigint::BigUint;

extern crate reqwest;

extern crate serde;
use serde::{Serialize, Deserialize};

const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize)]
struct FieldClaim {
    search_id: u32,
    base: u32,
    search_start: u128, // u128 will only get us to base 97
    search_end: u128,
    //claimed_time: String,
    //claimed_by: String,
    //expiration_time: String
}

#[derive(Debug, Serialize)]
struct FieldSubmit<'me> {
    search_id: u32,
    username: &'me str,
    client_version: &'static str,
    unique_count: HashMap<u32,u32>,
    near_misses: HashMap<u128,u32>
}

// get the number of unique digits in the concatenated sqube of a specified number
fn get_num_uniques(num: u128, base: u32) -> u32 {

    // sqube: the list of numbers in the quare and cube
    // initialized here with just the square
    let mut sqube = BigUint::from(num)
        .pow(2)
        .to_radix_be(base);
    
    // concatenate in the cube values
    sqube.append(&mut BigUint::from(num)
        .pow(3)
        .to_radix_be(base));
    
    // sort & dedup to get just the unique values
    sqube.sort();
    sqube.dedup();

    // return the length of the deduplicated list
    return sqube.len() as u32;
}

#[test]
fn test_get_num_uniques() {
    assert_eq!(
        get_num_uniques(69, 10), 
        10
    );
    assert_eq!(
        get_num_uniques(256, 2), 
        2
    );
    assert_eq!(
        get_num_uniques(123, 8), 
        8
    );
    assert_eq!(
        get_num_uniques(15, 16), 
        5
    );
    assert_eq!(
        get_num_uniques(100, 99), 
        3
    );
    assert_eq!(
        get_num_uniques(4134931983708, 40), 
        39
    );
    assert_eq!(
        get_num_uniques(173583337834150, 44), 
        41
    );
}

// get niceness data on a range of numbers and aggregate it
fn search_range(n_start: u128, n_end: u128, base: u32) -> (Vec<u128>,HashMap<u32,u32>) {

    // near_misses_cutoff: minimum number of uniques required for the nbumber to be recorded
    let near_misses_cutoff: f32 = base as f32 * 0.9;

    // near_misses: list of numbers with niceness ratio (uniques/base) above the cutoff
    // pre-allocate memory for the maximum possible number of near misses (wastes memory but saves resizing)
    let mut near_misses: Vec<u128> = Vec::with_capacity((n_end - n_start) as usize); 
    
    // qty_uniques: the quantity of numbers with each possible niceness
    let mut qty_uniques: HashMap<u32,u32> = HashMap::new(); 

    // build the initial values (api expects it)
    for b in 1..base+1 { 
        qty_uniques.insert(b,0);
    }

    // loop for all items in range
    for num in n_start..n_end { 

        // get the number of uniques in the sqube
        let num_uniques: u32 = get_num_uniques(num, base);

        // check if it's nice enough to record in near_misses
        if num_uniques as f32 > near_misses_cutoff {
            near_misses.push(num);
        }

        // update our quantity distribution in qty_uniques
        qty_uniques.insert(
            num_uniques, 
            qty_uniques.get(&num_uniques).copied().unwrap_or(0)+1
        );
    }

    // return it as a tuple
    return (near_misses,qty_uniques)
}

#[test]
fn test_search_range() {
    assert_eq!(
        search_range(47, 100, 10),
        (
            Vec::from([
                69,
            ]),
            HashMap::from([
                (1, 0),
                (2, 0),
                (3, 0),
                (4, 4),
                (5, 5),
                (6, 15),
                (7, 20),
                (8, 7),
                (9, 1),
                (10, 1),
            ])
        )
    );
    assert_eq!(
        search_range(144, 329, 12),
        (
            Vec::from([]),
            HashMap::from([
                (1, 0),
                (2, 1),
                (3, 1),
                (4, 6),
                (5, 15),
                (6, 27),
                (7, 55),
                (8, 53),
                (9, 24),
                (10, 3),
                (11, 0),
                (12, 0),
            ])
        )
    );
}

// get the claim data from the server
fn get_claim_data(username: &str) -> FieldClaim {
    let query_url = "https://nice.wasabipesto.com/claim?username=".to_owned() + &username;
    let claim_data: Result<FieldClaim, reqwest::Error> = reqwest::blocking::get(query_url).unwrap().json();
    return claim_data.unwrap();
}

fn main() {
    // get username from first argument
    let mut args = env::args();
    let username = args.by_ref().skip(1).next().unwrap_or_else(|| {
        "anonymous".to_string()
    });

    // get search data
    let claim_data = get_claim_data(&username);
    println!("{:?}", claim_data);

    // search for near_misses and qty_uniques
    let (
        near_misses, 
        qty_uniques
    ) = search_range(
        claim_data.search_start,
        claim_data.search_end,
        claim_data.base,
    );
    
    let mut near_miss_map: HashMap<u128,u32> = HashMap::new();
    for nm in near_misses.iter() {
        near_miss_map.insert(
            *nm,
            get_num_uniques(
                *nm,
                claim_data.base
            )
        );
    }

    // compile results
    let submit_data = FieldSubmit { 
        search_id: claim_data.search_id,
        username: &username,
        client_version: &CLIENT_VERSION,
        unique_count: qty_uniques,
        near_misses: near_miss_map
    };
    println!("{:?}", submit_data);
    
    // upload results
    let client = reqwest::blocking::Client::new();
    let _response = client.post("https://nice.wasabipesto.com/submit")
        .json(&submit_data)
        .send();

    // show response (debug)
    //println!("{:?}", response);
}