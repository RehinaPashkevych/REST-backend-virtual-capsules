use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::http::Status;
use rocket::response::status;
use chrono::{DateTime, Utc};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use sha2::{Sha256, Digest};
use rocket::response::status::Custom;
use std::collections::HashMap;

use crate::contributors::CONTRIBUTORS;
use crate::items::ITEMS;

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Capsule {
    pub id: u32,
    pub contributor_id: u32,
    pub name: String,
    pub description: String,
    pub time_created: DateTime<Utc>,
    pub time_changed: Option<DateTime<Utc>>,
    pub time_open: DateTime<Utc>,
    pub time_until_changed: DateTime<Utc>, // Time until the capsule can be changed
    pub item_ids: Option<Vec<u32>>, 
    pub idempotency_key: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CapsulePatch {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct NewCapsule {
    name: String,
    description: String,
    contributor_id: u32,
    time_open: DateTime<Utc>,
}

#[derive(FromForm, UriDisplayQuery)]
pub struct Pagination {
    page: Option<usize>,
    per_page: Option<usize>,
}


// Global in-memory storage for capsules
pub static CAPSULES: Lazy<Mutex<Vec<Capsule>>> = Lazy::new(|| {
    Mutex::new(vec![])
});

static IDEMPOTENCY_RECORDS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));


fn generate_idempotency_key(item: &NewCapsule) -> String {
    let mut hasher = Sha256::new();
    hasher.update(&item.name);
    hasher.update(&item.description);
    hasher.update(item.contributor_id.to_string());
    hasher.update(item.time_open.to_rfc3339());  // Convert DateTime to string

    format!("{:x}", hasher.finalize())
}

#[post("/capsules", format = "json", data = "<capsule_data>")]
pub fn create_capsule(capsule_data: Json<NewCapsule>) -> Result<Json<Capsule>, status::Custom<Json<String>>> {
    let mut contributors = CONTRIBUTORS.lock().unwrap();
    let mut capsules = CAPSULES.lock().unwrap();

    let new_capsule = capsule_data.into_inner();

    let mut idempotency_records = IDEMPOTENCY_RECORDS.lock().unwrap();

    // Generate the idempotency key
    let idempotency_key = generate_idempotency_key(&new_capsule);

    // Check for existing idempotency key to avoid processing the same request multiple times
    if idempotency_records.contains_key(&idempotency_key) {
        return Err(status::Custom(Status::BadRequest, Json("Duplicate submission detected".into())));
    }

    if contributors.iter().any(|c| c.id == new_capsule.contributor_id) {
        let id = capsules.iter().max_by_key(|c| c.id).map_or(1, |max| max.id + 1); // Ensure unique ID

        let capsule = Capsule {
            id,
            name: new_capsule.name.clone(),  // Clone to avoid move
            description: new_capsule.description.clone(),  // Clone to avoid move
            time_created: Utc::now(),
            time_changed: None,
            time_open: new_capsule.time_open,
            time_until_changed: Utc::now() + chrono::Duration::weeks(1),
            contributor_id: new_capsule.contributor_id,
            item_ids: None,
            idempotency_key: idempotency_key.clone(),
        };

        capsules.push(capsule.clone());

        // Record the successful operation to handle future idempotency
        idempotency_records.insert(idempotency_key, serde_json::to_string(&capsule).unwrap());

        // Update the contributor's list of capsule IDs
        if let Some(contributor) = contributors.iter_mut().find(|c| c.id == new_capsule.contributor_id) {
            if let Some(capsule_ids) = &mut contributor.capsule_ids {
                capsule_ids.push(capsule.id);
            } else {
                contributor.capsule_ids = Some(vec![capsule.id]);
            }
        }

        Ok(Json(capsule))
    } else {
        Err(status::Custom(Status::BadRequest, Json("Contributor not found".to_string())))
    }
}




#[get("/capsules?<pagination..>")]
pub fn list_capsules(pagination: Pagination) -> Result<Json<Vec<Capsule>>, Status> {
    let capsules = CAPSULES.lock().map_err(|_| Status::InternalServerError)?;

    let per_page = pagination.per_page.unwrap_or(10); // Default to 10 items per page if not specified
    let page = pagination.page.unwrap_or(1); // Default to page 1 if not specified
    let start = (page - 1) * per_page;
    let end = start + per_page;

    let paged_capsules = capsules[start..end.min(capsules.len())].to_vec(); // Safely slice the vector to the page size, handling cases where the range may exceed the vector bounds

    Ok(Json(paged_capsules))
}

#[get("/capsules/<cid>")]
pub fn capsule_detail(cid: u32) -> Result<Option<Json<Capsule>>, Status> {
    let capsules = CAPSULES.lock().map_err(|_| Status::InternalServerError)?;
    Ok(capsules.iter().find(|c| c.id == cid).cloned().map(Json))
}

#[put("/capsules/<cid>", format = "json", data = "<capsule_data>")]
pub fn update_capsule(cid: u32, capsule_data: Json<Capsule>) -> Result<Option<Json<Capsule>>, status::Custom<Json<String>>> {
    let mut capsules = CAPSULES.lock().unwrap();

    if let Some(capsule) = capsules.iter_mut().find(|c| c.id == cid) {
        if Utc::now() > capsule.time_until_changed {
            return Err(status::Custom(Status::BadRequest, Json("The modification period for this capsule has expired".to_string())));
        }
        *capsule = capsule_data.into_inner();
        capsule.time_changed = Some(Utc::now());
        Ok(Some(Json(capsule.clone())))
    } else {
        Err(status::Custom(Status::NotFound, Json("Capsule not found".to_string())))
    }
}


#[patch("/capsules/<cid>", format = "json", data = "<capsule_data>")]
pub fn patch_capsule(cid: u32, capsule_data: Json<CapsulePatch>) -> Result<Json<Capsule>, status::Custom<Json<String>>> {
    let mut capsules = CAPSULES.lock().unwrap();

    if let Some(capsule) = capsules.iter_mut().find(|c| c.id == cid) {
        if Utc::now() > capsule.time_until_changed {
            return Err(status::Custom(Status::BadRequest, Json("The modification period for this capsule has expired".to_string())));
        }

        // Apply changes as before, now with the time check
        let time_now = Utc::now();
        let mut updated = false;

        // Check if the `name` field has a value and if so, assign it
        if let Some(ref name) = capsule_data.name {
            capsule.name = name.clone();  // Clone to avoid moving the value
            updated = true;
        }

        // Check if the `description` field has a value and if so, assign it
        if let Some(ref description) = capsule_data.description {
            capsule.description = description.clone();  // Clone to avoid moving the value
            updated = true;
        }

        if updated {
            capsule.time_changed = Some(time_now);
            Ok(Json(capsule.clone()))
        } else {
            Err(status::Custom(Status::BadRequest, Json("No valid fields provided for update".to_string())))
        }
    } else {
        Err(status::Custom(Status::NotFound, Json("Capsule not found".to_string())))
    }
}


#[delete("/capsules/<cid>")]
pub fn delete_capsule(cid: u32) -> Result<Status, status::Custom<Json<String>>> {
    let mut capsules = CAPSULES.lock().unwrap();
    let mut items = ITEMS.lock().unwrap(); // Lock the items data
    let mut contributors = CONTRIBUTORS.lock().unwrap();

    if let Some(index) = capsules.iter().position(|c| c.id == cid) {
        let contributor_id = capsules[index].contributor_id;

        // Retrieve the item IDs before removing the capsule
        let item_ids_to_remove = if let Some(item_ids) = &capsules[index].item_ids {
            item_ids.clone()
        } else {
            Vec::new()
        };

        // Remove the capsule
        capsules.remove(index);

        // Remove all items that belong to this capsule
        items.retain(|item| !item_ids_to_remove.contains(&item.id));

        // Update the contributor's list of capsule IDs
        if let Some(contributor) = contributors.iter_mut().find(|c| c.id == contributor_id) {
            if let Some(capsule_ids) = &mut contributor.capsule_ids {
                let pos = capsule_ids.iter().position(|&x| x == cid);
                if let Some(pos) = pos {
                    capsule_ids.remove(pos);
                }
            }
        }

        Ok(Status::NoContent)
    } else {
        Err(status::Custom(Status::NotFound, Json("Capsule not found".to_string())))
    }
}
