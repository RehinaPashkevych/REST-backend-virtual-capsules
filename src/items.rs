use rocket::serde::{Serialize, Deserialize, json::Json};
use chrono::{DateTime, Utc};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use rocket::response::status;
use rocket::http::{Status};

use crate::capsules::CAPSULES;

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Item {
    pub id: u32,  // Now public, allowing access from other modules
    pub id_capsule: u32,
    pub type_c: String,
    pub time_added: DateTime<Utc>,
    pub description: String,
    pub size: String,
    pub path: String,
    pub metadata: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NewItem {
    pub type_c: String,
    pub description: String,
    pub size: String,
    pub path: String,
    pub metadata: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NewItemUpdate {
    pub description: String,
}

// Global in-memory storage for items
pub static ITEMS: Lazy<Mutex<Vec<Item>>> = Lazy::new(|| {
    Mutex::new(vec![])
});



#[get("/items")]
pub fn get_all_items() -> Result<Json<Vec<Item>>, Status> {
    let items = ITEMS.lock().map_err(|_| Status::InternalServerError)?;
    Ok(Json(items.clone()))
}

#[get("/items/<item_id>")]
pub fn get_item(item_id: u32) -> Result<Json<Item>, status::Custom<Json<String>>> {
    let items = ITEMS.lock().unwrap();

    match items.iter().find(|item| item.id == item_id) {
        Some(item) => Ok(Json(item.clone())),
        None => Err(status::Custom(Status::NotFound, Json(format!("Item with ID {} not found", item_id))))
    }
}


#[get("/capsules/<cid>/items")]
pub fn get_capsule_items(cid: u32) -> Result<Json<Vec<Item>>, status::Custom<Json<String>>> {
    let capsules = CAPSULES.lock().unwrap();
    let items = ITEMS.lock().unwrap();

    // Find the capsule by ID and retrieve associated items
    if let Some(capsule) = capsules.iter().find(|&c| c.id == cid) {
        if let Some(item_ids) = &capsule.item_ids {
            let capsule_items: Vec<Item> = item_ids
                .iter()
                .filter_map(|id| items.iter().find(|&item| item.id == *id))
                .cloned()
                .collect();
            
            Ok(Json(capsule_items))
        } else {
            // Handle case where capsule has no items
            Ok(Json(vec![]))
        }
    } else {
        Err(status::Custom(Status::NotFound, Json(format!("No capsule found with ID {}", cid))))
    }
}


#[post("/capsules/<cid>/items", format = "json", data = "<item_data>")]
pub fn add_item_to_capsule(cid: u32, item_data: Json<NewItem>) -> Result<Json<Item>, status::Custom<Json<String>>> {
    let mut items = ITEMS.lock().unwrap();
    let mut capsules = CAPSULES.lock().unwrap();

    // Generate a new ID for the item
    let new_id = items.iter().max_by_key(|item| item.id).map_or(1, |max_item| max_item.id + 1);

    // Create new item with new ID and current timestamp
    let new_item = Item {
        id: new_id,
        id_capsule: cid,
        type_c: item_data.type_c.clone(),
        description: item_data.description.clone(),
        size: item_data.size.clone(),
        path: item_data.path.clone(),
        metadata: item_data.metadata.clone(),
        time_added: Utc::now(),
    };

    // Add the new item to the ITEMS list
    items.push(new_item.clone());

    // Update the corresponding capsule's item_ids list
    if let Some(capsule) = capsules.iter_mut().find(|cap| cap.id == cid) {
        if let Some(ref mut item_ids) = capsule.item_ids {
            item_ids.push(new_id);
        } else {
            capsule.item_ids = Some(vec![new_id]);
        }
    } else {
        return Err(status::Custom(Status::NotFound, Json(format!("Capsule with ID {} not found", cid))));
    }

    Ok(Json(new_item))
}

#[get("/capsules/<capsule_id>/items/<item_id>")]
pub fn get_capsule_item(capsule_id: u32, item_id: u32) -> Result<Json<Item>, status::Custom<Json<String>>> {
    let capsules = CAPSULES.lock().unwrap();
    let items = ITEMS.lock().unwrap();

    if let Some(capsule) = capsules.iter().find(|&c| c.id == capsule_id) {
        if capsule.item_ids.as_ref().map_or(false, |ids| ids.contains(&item_id)) {
            if let Some(item) = items.iter().find(|&item| item.id == item_id) {
                return Ok(Json(item.clone()));
            }
        }
    }
    Err(status::Custom(Status::NotFound, Json("Item not found in the specified capsule".to_string())))
}


#[patch("/capsules/<capsule_id>/items/<item_id>", format = "json", data = "<item_update>")]
pub fn patch_capsule_item_description(capsule_id: u32, item_id: u32, item_update: Json<NewItemUpdate>) -> Result<Json<Item>, status::Custom<Json<String>>> {
    let mut items = ITEMS.lock().unwrap();
    let capsules = CAPSULES.lock().unwrap();

    // First, verify the capsule contains the item
    if let Some(capsule) = capsules.iter().find(|&c| c.id == capsule_id && c.item_ids.as_ref().map_or(false, |ids| ids.contains(&item_id))) {
        // Find the item and update the description
        if let Some(item) = items.iter_mut().find(|item| item.id == item_id) {
            item.description = item_update.description.clone();
            return Ok(Json(item.clone()));
        }
    }
    Err(status::Custom(Status::NotFound, Json(format!("No item with ID {} found in capsule {}", item_id, capsule_id))))
}


#[delete("/capsules/<capsule_id>/items/<item_id>")]
pub fn delete_capsule_item(capsule_id: u32, item_id: u32) -> Result<Status, status::Custom<Json<String>>> {
    let mut items = ITEMS.lock().unwrap();
    let mut capsules = CAPSULES.lock().unwrap();

    // Verify the item is in the specified capsule
    if let Some(capsule) = capsules.iter_mut().find(|cap| cap.id == capsule_id) {
        if let Some(pos) = capsule.item_ids.as_mut().unwrap().iter().position(|&id| id == item_id) {
            // Remove the item ID from the capsule's item_ids list
            capsule.item_ids.as_mut().unwrap().remove(pos);

            // Remove the item from the ITEMS list
            items.retain(|item| item.id != item_id);

            return Ok(Status::NoContent);
        }
    }
    Err(status::Custom(Status::NotFound, Json(format!("Item with ID {} not found in capsule {}", item_id, capsule_id))))
}

