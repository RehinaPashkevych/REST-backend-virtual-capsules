use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::http::Status;
use rocket::response::status;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// Assume these are in a module named `capsules`
use crate::capsules::{Capsule, CAPSULES};
use crate::items::ITEMS;


#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Contributor {
    pub id: u32,
    pub capsule_ids: Option<Vec<u32>>,
    pub name: String,
    pub email: String,
    
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NewContributor {
    pub name: String,
    pub email: String,
    // No `id_capsule` since it might not be set at creation
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ContributorCapsules {
    contributor: Contributor,
    capsules: Vec<Capsule>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ContributorUpdate {
    pub name: Option<String>,
    pub email: Option<String>,
}


#[derive(FromForm, UriDisplayQuery)]
pub struct Pagination {
    page: Option<usize>,
    per_page: Option<usize>,
}



// This would typically be stored in a database
pub static CONTRIBUTORS: Lazy<Mutex<Vec<Contributor>>> = Lazy::new(|| {
    Mutex::new(vec![])
});


#[post("/contributors", format = "json", data = "<contributor_data>")]
pub fn create_contributor(contributor_data: Json<NewContributor>) -> Result<Json<Contributor>, status::Custom<Json<String>>> {
    let mut contributors = CONTRIBUTORS.lock().unwrap();
    let new_contributor = contributor_data.into_inner();

    // Check if the email already exists
    if contributors.iter().any(|c| c.email == new_contributor.email) {
        return Err(status::Custom(Status::Conflict, Json("Email already in use".to_string())));
    }

    let id = contributors.len() as u32 + 1;
    let contributor = Contributor {
        id,
        name: new_contributor.name,
        email: new_contributor.email,
        capsule_ids: None, 
    };
    contributors.push(contributor.clone());
    Ok(Json(contributor))
}


#[get("/contributors?<pagination..>")]
pub fn list_contributors(pagination: Pagination) -> Result<Json<Vec<Contributor>>, Status> {
    let contributors = CONTRIBUTORS.lock().map_err(|_| Status::InternalServerError)?;

    let per_page = pagination.per_page.unwrap_or(10); // Default to 10 items per page if not specified
    let page = pagination.page.unwrap_or(1); // Default to page 1 if not specified
    let start = (page - 1) * per_page;
    let end = start + per_page;

    let paged_contributors = contributors[start..end.min(contributors.len())].to_vec(); // Safely slice the vector to the page size, handling cases where the range may exceed the vector bounds

    Ok(Json(paged_contributors))
}

#[get("/contributors/<contributor_id>")]
pub fn get_contributor_with_capsules(contributor_id: u32) -> Result<Json<ContributorCapsules>, status::Custom<Json<String>>> {
    let contributors = CONTRIBUTORS.lock().unwrap();
    let capsules = CAPSULES.lock().unwrap();

    if let Some(contributor) = contributors.iter().find(|c| c.id == contributor_id) {
        if let Some(ref capsule_ids) = contributor.capsule_ids {
            let contributor_capsules = capsule_ids.iter()
                .filter_map(|id| capsules.iter().find(|c| c.id == *id))
                .cloned()
                .collect::<Vec<Capsule>>();

            Ok(Json(ContributorCapsules {
                contributor: contributor.clone(),
                capsules: contributor_capsules
            }))
        } else {
            // Handle the case where there are no capsule IDs
            Ok(Json(ContributorCapsules {
                contributor: contributor.clone(),
                capsules: Vec::new() // No capsules associated
            }))
        }
    } else {
        Err(status::Custom(Status::NotFound, Json("Contributor not found".to_string())))
    }
}

#[patch("/contributors/<id>", format = "json", data = "<contributor_data>")]
pub fn update_contributor(id: u32, contributor_data: Json<ContributorUpdate>) -> Result<Json<Contributor>, status::Custom<Json<String>>> {
    let mut contributors = CONTRIBUTORS.lock().unwrap();

    // First, determine if the new email is provided and needs to be unique
    if let Some(ref new_email) = contributor_data.email {
        // Check for email uniqueness
        if contributors.iter().any(|c| c.id != id && c.email == *new_email) {
            return Err(status::Custom(Status::Conflict, Json("Email already in use".to_string())));
        }
    }

    // Now proceed with finding and updating the contributor
    if let Some(contributor) = contributors.iter_mut().find(|c| c.id == id) {
        // Update name if provided
        if let Some(ref name) = contributor_data.name {
            contributor.name = name.clone();
        }

        // Update email if provided and checked
        if let Some(ref new_email) = contributor_data.email {
            contributor.email = new_email.clone();  // Cloning the string here
        }

        Ok(Json(contributor.clone()))
    } else {
        Err(status::Custom(Status::NotFound, Json("Contributor not found".to_string())))
    }
}


#[delete("/contributors/<contributor_id>")]
pub fn delete_contributor(contributor_id: u32) -> Result<Status, status::Custom<Json<String>>> {
    let mut contributors = CONTRIBUTORS.lock().unwrap();
    let mut capsules = CAPSULES.lock().unwrap();
    let mut items = ITEMS.lock().unwrap();  // Lock the items data

    // First, find if the contributor exists
    if let Some(pos) = contributors.iter().position(|c| c.id == contributor_id) {
        // Remove the contributor
        contributors.remove(pos);

        // Collect all capsule IDs associated with the contributor
        let capsule_ids_to_remove: Vec<u32> = capsules.iter()
            .filter(|capsule| capsule.contributor_id == contributor_id)
            .map(|capsule| capsule.id)
            .collect();

        // Now remove all capsules associated with this contributor
        capsules.retain(|capsule| capsule.contributor_id != contributor_id);

        // Remove all items that belong to the capsules of the deleted contributor
        items.retain(|item| !capsule_ids_to_remove.contains(&item.id_capsule));

        Ok(Status::NoContent)
    } else {
        Err(status::Custom(Status::NotFound, Json("Contributor not found".to_string())))
    }
}
