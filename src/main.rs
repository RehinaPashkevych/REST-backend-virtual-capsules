#[macro_use] extern crate rocket;
use std::fs;
use serde_json; // Ensure serde_json is added to your dependencies

mod capsules;
use capsules::{create_capsule, list_capsules, capsule_detail, update_capsule, patch_capsule, delete_capsule};

mod contributors;
use contributors::{create_contributor, list_contributors, get_contributor_with_capsules, delete_contributor,
    update_contributor};

mod items;
use items::{get_all_items, get_item, get_capsule_items, add_item_to_capsule, get_capsule_item,
    patch_capsule_item_description, delete_capsule_item};

mod merges;
use merges::{merge_capsules };

#[launch]
fn rocket() -> _ {
    let contributors_json = fs::read_to_string("C:/Users/РЕГИНА/Desktop/studia/RUST/rest-capsules/src/data/contributors.json").expect("Failed to read contributors.json");
    let capsules_json = fs::read_to_string("C:/Users/РЕГИНА/Desktop/studia/RUST/rest-capsules/src/data/capsule.json").expect("Failed to read capsules.json");
    let items_json = fs::read_to_string("C:/Users/РЕГИНА/Desktop/studia/RUST/rest-capsules/src/data/items.json").expect("Failed to read items.json");

    let contributors_data: Vec<contributors::Contributor> = serde_json::from_str(&contributors_json).expect("Invalid format in contributors.json");
    let capsules_data: Vec<capsules::Capsule> = serde_json::from_str(&capsules_json).expect("Invalid format in capsules.json");
    let items_data: Vec<items::Item> = serde_json::from_str(&items_json).expect("Invalid format in items.json");

    // Fill the global state with data loaded from files
    *contributors::CONTRIBUTORS.lock().unwrap() = contributors_data;
    *capsules::CAPSULES.lock().unwrap() = capsules_data;
    *items::ITEMS.lock().unwrap() = items_data;

    rocket::build()
        .mount("/", routes![
            create_capsule, list_capsules, capsule_detail, update_capsule, patch_capsule, delete_capsule,
            create_contributor, list_contributors, get_contributor_with_capsules, delete_contributor, update_contributor,
            get_all_items, get_item, get_capsule_items, add_item_to_capsule, get_capsule_item,
            patch_capsule_item_description, delete_capsule_item,
            merge_capsules
        ])
}
