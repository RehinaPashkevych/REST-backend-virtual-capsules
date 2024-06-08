use rocket::serde::{Serialize, Deserialize, json::Json};
use chrono::{DateTime, Utc};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use rocket::response::status;
use rocket::http::{Status, ContentType};
use rocket::response::status::Custom;
use rocket::response::{Response, Responder, self};
use std::collections::HashMap;
use rocket::Request;

use crate::capsules::{ CAPSULES};
use sha2::{Sha256, Digest};

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
 //   pub idempotency_key: String,
    pub version: u32,
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
    version: Option<u32>,
}


#[derive(FromForm, UriDisplayQuery)]
pub struct Pagination {
    page: Option<usize>,
    per_page: Option<usize>,
}


// Global in-memory storage for items
pub static ITEMS: Lazy<Mutex<Vec<Item>>> = Lazy::new(|| {
    Mutex::new(vec![])
});

//static IDEMPOTENCY_RECORDS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/*
fn generate_idempotency_key(item: &NewItem) -> String {
    let mut hasher = Sha256::new();
    hasher.update(&item.type_c);
    hasher.update(&item.description);
    hasher.update(&item.size);
    hasher.update(&item.path);
    hasher.update(serde_json::to_string(&item.metadata).unwrap());
    format!("{:x}", hasher.finalize())
}*/


// Define a custom responder that includes pagination information
// Custom responder to add headers
pub struct CustomResponder<T> {
    inner: T,
    total_items: usize,
    page: usize,
    per_page: usize,
}

impl<'r, T: Responder<'r, 'static>> Responder<'r, 'static> for CustomResponder<T> {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        let mut build = Response::build_from(self.inner.respond_to(request)?);
        build.raw_header("X-Total-Count", self.total_items.to_string());
        build.raw_header("X-Page", self.page.to_string());
        build.raw_header("X-Per-Page", self.per_page.to_string());
        build.ok()
    }
}



#[get("/items?<pagination..>")]
pub fn get_all_items(pagination: Pagination) ->  Result<CustomResponder<Json<Vec<Item>>>, Status> {
    let items = ITEMS.lock().map_err(|_| Status::InternalServerError)?;

    let per_page = pagination.per_page.unwrap_or(10); // Default to 10 items per page if not specified
    let page = pagination.page.unwrap_or(1); // Default to page 1 if not specified
    let start = (page - 1) * per_page;
    let end = start + per_page;

    let paged_items = items[start..end.min(items.len())].to_vec(); // Safely slice the vector to the page size

    Ok(CustomResponder {
        inner: Json(paged_items),
        total_items: items.len(),
        page,
        per_page,
    })
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
pub fn add_item_to_capsule(cid: u32, item_data: Json<NewItem>) -> Result<Json<Item>, Custom<Json<String>>> {
    let mut items = ITEMS.lock().unwrap();
    let mut capsules = CAPSULES.lock().unwrap();
   // let mut idempotency_records = IDEMPOTENCY_RECORDS.lock().unwrap();

    // Generate the idempotency key
   // let idempotency_key = generate_idempotency_key(&item_data);

    // Check for existing idempotency key to avoid processing the same request multiple times
    //if idempotency_records.contains_key(&idempotency_key) {
   //     return Err(Custom(Status::BadRequest, Json("Duplicate item submission detected".into())));
    //}

    // Find the corresponding capsule
    if let Some(capsule) = capsules.iter_mut().find(|cap| cap.id == cid) {
        // Check if the capsule modification period has expired
        if Utc::now() > capsule.time_until_changed {
            return Err(Custom(Status::BadRequest, Json("The modification period for this capsule has expired".into())));
        }

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
            //idempotency_key: idempotency_key.clone(),
            version: 1
        };

        // Update the capsule's item list and modification time
        capsule.item_ids.get_or_insert_with(Vec::new).push(new_id);
        capsule.time_changed = Some(Utc::now());

        // Add the new item to the global list
        items.push(new_item.clone());

        // Record the successful operation to handle future idempotency
      //  idempotency_records.insert(idempotency_key, serde_json::to_string(&new_item).unwrap());

        Ok(Json(new_item))
    } else {
        Err(Custom(Status::NotFound, Json(format!("Capsule with ID {} not found", cid))))
    }
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


#[patch("/capsules/<capsule_id>/items/<item_id>?<etag>", format = "json", data = "<item_update>")]
pub fn patch_capsule_item_description(
    capsule_id: u32, 
    item_id: u32, 
    etag: Option<u32>, 
    item_update: Json<NewItemUpdate>
) -> Result<Json<Item>, status::Custom<Json<String>>> {
    let mut items = ITEMS.lock().unwrap();
    let mut capsules = CAPSULES.lock().unwrap();

    // Verify the capsule contains the item and can still be changed
    if let Some(capsule) = capsules.iter_mut().find(|c| c.id == capsule_id && c.item_ids.as_ref().map_or(false, |ids| ids.contains(&item_id))) {
        if Utc::now() > capsule.time_until_changed {
            return Err(status::Custom(Status::BadRequest, Json("The modification period for this capsule has expired".into())));
        }

        if let Some(item) = items.iter_mut().find(|item| item.id == item_id) {
            // Resolve version to check from ETag or the update body
            let version_to_check = etag.or(item_update.version);

            // Check version matches
            if version_to_check.is_none() || version_to_check != Some(item.version) {
                return Err(status::Custom(Status::Conflict, Json("Version mismatch. Please refresh your data.".into())));
            }

            // Proceed with the update
            item.description = item_update.description.clone();
            item.version += 1;  // Increment the version to signify an update
            capsule.time_changed = Some(Utc::now());  // Update the capsule's last modified time

            return Ok(Json(item.clone()));
        }
    }

    Err(status::Custom(Status::NotFound, Json(format!("No item with ID {} found in capsule {}", item_id, capsule_id))))
}




#[delete("/capsules/<capsule_id>/items/<item_id>")]
pub fn delete_capsule_item(capsule_id: u32, item_id: u32) -> Result<Status, status::Custom<Json<String>>> {
    let mut items = ITEMS.lock().unwrap();
    let mut capsules = CAPSULES.lock().unwrap();

    // Verify the capsule can still be changed and contains the specified item
    if let Some(capsule) = capsules.iter_mut().find(|cap| cap.id == capsule_id) {
        if Utc::now() > capsule.time_until_changed {
            return Err(status::Custom(Status::BadRequest, Json("The modification period for this capsule has expired".into())));
        }

        if let Some(pos) = capsule.item_ids.as_mut().unwrap().iter().position(|&id| id == item_id) {
            // Remove the item ID from the capsule's item_ids list
            capsule.item_ids.as_mut().unwrap().remove(pos);
            // Remove the item from the ITEMS list
            items.retain(|item| item.id != item_id);
            capsule.time_changed = Some(Utc::now());  // Update the time_changed to now

            return Ok(Status::NoContent);
        }
    }
    Err(status::Custom(Status::NotFound, Json(format!("Item with ID {} not found in capsule {}", item_id, capsule_id))))
}