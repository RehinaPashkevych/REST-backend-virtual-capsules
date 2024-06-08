use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::http::{Status, Header};
use rocket::response::{self, Responder, Response, Redirect};
use rocket::Request;
use chrono::{DateTime, Utc};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use rocket::response::status;

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
    pub version: u32,  // Version counter to handle concurrent updates
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CapsulePatch {
    name: Option<String>,
    description: Option<String>,
    version: Option<u32>, 
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
/*
#[post("/capsules", format = "json", data = "<capsule_data>")]
pub fn create_capsule(capsule_data: Json<NewCapsule>) -> Result<Json<Capsule>, status::Custom<Json<String>>> {
    let mut contributors = CONTRIBUTORS.lock().unwrap();
    let mut capsules = CAPSULES.lock().unwrap();

    let new_capsule = capsule_data.into_inner();

    // Generate the idempotency key
   // let idempotency_key = generate_idempotency_key(&new_capsule);

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
            //idempotency_key: idempotency_key.clone(),
            version: 1,
        };

        capsules.push(capsule.clone());

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
}*/


#[post("/capsules", format = "json", data = "<capsule_data>")]
pub fn create_and_update_capsule(capsule_data: Json<NewCapsule>) -> Result<Json<Capsule>, status::Custom<Json<String>>> {
    let mut contributors = CONTRIBUTORS.lock().unwrap();
    let mut capsules = CAPSULES.lock().unwrap();

    let new_capsule = capsule_data.into_inner();

    // Check for contributor existence
    if !contributors.iter().any(|c| c.id == new_capsule.contributor_id) {
        return Err(status::Custom(Status::BadRequest, Json("Contributor not found".into())));
    }

    // Generate a unique ID for the new capsule
    let id = capsules.iter().max_by_key(|c| c.id).map_or(1, |max| max.id + 1);

    // Create the capsule with placeholder data
    let mut capsule = Capsule {
        id,
        name: new_capsule.name.clone(),  // Initial data from POST
        description: new_capsule.description.clone(),  // Initial data from POST
        time_created: Utc::now(),
        time_changed: None,
        time_open: new_capsule.time_open,
        time_until_changed: Utc::now() + chrono::Duration::weeks(1),
        contributor_id: new_capsule.contributor_id,
        item_ids: None,
        version: 1,
    };

    // Simulate a PUT operation by updating the newly created capsule immediately !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
    capsule.name = new_capsule.name.clone();
    capsule.description = new_capsule.description.clone();
    capsule.time_changed = Some(Utc::now());  // Update modification time

    // Add to the list of capsules
    capsules.push(capsule.clone());

    // Update the contributor's list of capsule IDs
    if let Some(contributor) = contributors.iter_mut().find(|c| c.id == new_capsule.contributor_id) {
        if let Some(capsule_ids) = &mut contributor.capsule_ids {
            capsule_ids.push(capsule.id);
        } else {
            contributor.capsule_ids = Some(vec![capsule.id]);
        }
    }

    Ok(Json(capsule))
}




#[get("/capsules?<pagination..>")]
pub fn list_capsules(pagination: Pagination) -> Result<CustomResponder<Json<Vec<Capsule>>>, Status> {
    let capsules = CAPSULES.lock().map_err(|_| Status::InternalServerError)?;

    let per_page = pagination.per_page.unwrap_or(10); // Default to 10 items per page if not specified
    let page = pagination.page.unwrap_or(1); // Default to page 1 if not specified
    let start = (page - 1) * per_page;
    let end = start + per_page;

    let paged_capsules = capsules[start..end.min(capsules.len())].to_vec(); // Safely slice the vector to the page size, handling cases where the range may exceed the vector bounds

    Ok(CustomResponder {
        inner: Json(paged_capsules),
        total_items: capsules.len(),
        page,
        per_page,
    })
}
/*
#[get("/capsules")]
pub fn redirect_to_default() -> Redirect {
    Redirect::to(uri!(list_capsules: Pagination { page: 1, per_page: 10 }))
}*/

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

#[patch("/capsules/<cid>?<etag>", format = "json", data = "<capsule_data>")]
pub fn patch_capsule(cid: u32, etag: Option<u32>, capsule_data: Json<CapsulePatch>) -> Result<Json<Capsule>, status::Custom<Json<String>>> {
    let mut capsules = CAPSULES.lock().unwrap();

    if let Some(capsule) = capsules.iter_mut().find(|c| c.id == cid) {
        if Utc::now() > capsule.time_until_changed {
            return Err(status::Custom(Status::BadRequest, Json("The modification period for this capsule has expired".into())));
        }

        // Determine the version to check against and handle conflicts if both are provided
        match (etag, capsule_data.version) {
            (Some(e), Some(v)) if e != v => {
                return Err(status::Custom(Status::BadRequest, Json("Conflicting versions provided. Please verify the ETag and JSON body version.".into())));
            },
            _ => {}
        }

        let version_to_check = etag.or(capsule_data.version);

        if let Some(version) = version_to_check {
            if capsule.version != version {
                return Err(status::Custom(Status::Conflict, Json("Version mismatch. Please refresh your data.".into())));
            }
        } else {
            return Err(status::Custom(Status::BadRequest, Json("Version number is required.".into())));
        }

        let time_now = Utc::now();
        let mut updated = false;

        if let Some(ref name) = capsule_data.name {
            capsule.name = name.clone();
            updated = true;
        }

        if let Some(ref description) = capsule_data.description {
            capsule.description = description.clone();
            updated = true;
        }

        if updated {
            capsule.time_changed = Some(time_now);
            capsule.version += 1; // Increment the version counter as the capsule has been updated.
            Ok(Json(capsule.clone()))
        } else {
            Err(status::Custom(Status::BadRequest, Json("No valid fields provided for update.".into())))
        }
    } else {
        Err(status::Custom(Status::NotFound, Json("Capsule not found.".into())))
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
