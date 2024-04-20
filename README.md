
# Digital Capsule Service

## Project Overview
This project serves as the backend component of a prototype system designed to manage digital "capsules" which contain various items like documents, images, and other types of digital content. Each capsule is associated with contributors who can perform operations such as creating, updating, or deleting capsules and their contents. The system is structured around a RESTful API, enabling interaction through HTTP requests.

#### Development and Testing:

*   The project is implemented in Rust, utilizing the Rocket framework for setting up the web server and endpoints.
*   For testing and interaction with the backend, a collection of prepared Postman queries is available. These queries can be used to simulate client requests to the backend and observe the system's behavior.
 https://api.postman.com/collections/28397225-8b16235c-4234-4fbb-9813-bf5e0e886224?access_key=PMAT-01HVW1Z95H1KYGSNA5S1R89WV7
*   It's important to note that this project is a backend-only prototype. The responses and functionalities are designed to demonstrate backend logic and data handling without an accompanying frontend interface.

## Features
- **Capsule Management**: Users can create, update, retrieve, and delete digital capsules.
- **Item Management**: Users can add items to capsules and manage these items.
- **Contributor Management**: Manage contributors who can own and modify capsules.
- **Merge Capsules**: Special functionality to merge two capsules into one.
- **Atomic Operations**: Ensures critical operations are performed without partial completion.

## API Endpoints

| Endpoint                        | Method   | Description                                      | Input Format         | Output Format        |
|---------------------------------|----------|--------------------------------------------------|----------------------|----------------------|
| `/capsules`                     | `GET`    | Retrieves all capsules                           | None                 | `List of Capsules`   |
| `/capsules`                     | `POST`   | Creates a new capsule                            | `Capsule Data`       | `Capsule`            |
| `/capsules/<cid>`               | `GET`    | Retrieves a specific capsule by ID               | None                 | `Capsule`            |
| `/capsules/<cid>`               | `PUT`    | Updates a specific capsule                       | `Capsule Data`       | `Capsule`            |
| `/capsules/<cid>`               | `DELETE` | Deletes a specific capsule                       | None                 | `Status`             |
| `/capsules/<cid>/items`         | `POST`   | Adds an item to a specific capsule               | `Item Data`          | `Item`               |
| `/capsules/<cid>/items/<iid>`   | `PATCH`  | Updates an item's description in a capsule       | `Item Description`   | `Item`               |
| `/capsules/<cid>/items/<iid>`   | `DELETE` | Removes an item from a capsule                   | None                 | `Status`             |
| `/contributors`                 | `GET`    | Retrieves all contributors                       | None                 | `List of Contributors` |
| `/contributors`                 | `POST`   | Adds a new contributor                           | `Contributor Data`   | `Contributor`        |
| `/contributors`                 | `PATCH`  | Updates a contributor`s name and email           | `Contributor Data`   | `Contributor`        |
| `/contributors/<cid>`           | `GET`    | Retrieves a specific contributor by ID           | None                 | `Contributor`        |
| `/contributors/<cid>`           | `DELETE` | Deletes a specific contributor                   | None                 | `Status`             |
| `/merges/<cid1>/<cid2>`         | `POST`   | Merges two capsules into one                     | None                 | `Capsule`            |
| `/merges `                      | `GET`    |Retrieves all merges                              | None                 | `Capsule`            |
| `/items`                        | `GET`    | Retrieves all items with optional pagination     | `Pagination Params`  | `List of Items`      |

There are query parameters for `/capsules`,  `/contributors`,  `/items` endpoints for GET method. The usage is:

```
http://127.0.0.1:8000/contributors?page=2&per_page=1
```

### POST Exactly-Once Implementation

In this project, exactly-once semantics are implemented to ensure that POST requests are idempotent. This means that multiple submissions of the same request will result in only one unique processing action, preventing duplicate data entries in the system. The mechanism is based on generating a unique idempotency key for each request, which is checked against a record of previously processed requests.

#### Implemented Routes

*   **POST `/capsules`**: This route generates a unique idempotency key based on the capsule's properties (name, description, contributor ID, and time open). If a request with the same key is received, the server will reject it with an error, indicating that the request has been detected as a duplicate.
*   **POST `/capsules/<cid>/items`**: Similar to capsule creation, this route generates an idempotency key for each item added to a capsule based on the item's properties (type, description, size, path, and metadata). This key helps prevent the addition of duplicate items to a capsule if the same request is sent multiple times.

#### Exception - POST for Contributors

*   **POST `/contributors`** does not implement the exactly-once mechanism via idempotency keys because it inherently checks for the uniqueness of the email address associated with each contributor. If a request attempts to add a contributor with an existing email, the system will reject the request based on the unique constraint of the email field, thus ensuring idempotency by design.

## Data Formats

### Capsule Data (Input)
```json
{
    "name": "Project Launch Details",
    "description": "Detailed plans for the upcoming project.",
    "contributor_id": 3,
    "time_open": "2044-04-12T11:45:00Z"
}
```

### Capsule (Output)
```json
{
    "id": 1,
    "contributor_id": 3,
    "name": "new_one_sec",
    "description": "Detailed plans for the upcoming project.",
    "time_created": "2024-04-19T14:34:18.709154800Z",
    "time_changed": null,
    "time_open": "2044-04-12T11:45:00Z",
    "time_until_changed": "2024-04-26T14:34:18.709155600Z",
    "item_ids": null
}
```

### Item Data (Input)
```json
{
    "type_c": "photo",
    "description": "Photo from New Year's Eve",
    "size": "2MB",
    "path": "path/to/photo1.jpg",
     "metadata": {
        "resolution": "1920x1080",
        "somth": "hgb"
    }
}
```

### Item (Output)
```json
{
    "id": 6,
    "id_capsule": 6,
    "type_c": "photo-new2-2",
    "time_added": "2024-04-19T14:35:27.572856300Z",
    "description": "Photo from New Year's Eve",
    "size": "2MB",
    "path": "path/to/photo1.jpg",
    "metadata": {
        "resolution": "1920x1080",
        "somth": "hgb"
    }
}
```

### Contributor Data (Input)
```json
{
    "name": "John Doe",
    "email": "john.doe@example.com"
}
```

### Contributor (Output)
```json
{
    "id": 10,
    "capsule_ids": null,
    "name": "John Doe",
    "email": "john.doe@example.com"
}
```

## Running the Project
1. Clone the repository.
2. Navigate to the project directory.
3. Build the project with `cargo build`.
4. Run the project using `cargo run`.
5. Access the API endpoints through a REST client or browser.

## Data Folder

 Each Rust source file in the src directory is responsible for specific parts of the application logic:


*   **`capsules.rs`**:
    
    *   **Purpose**: Manages the `Capsule` entities in the system, including their creation, modification, and deletion. It defines the structure of a capsule and handles operations directly related to capsules, such as adding or modifying content.
    *   **Key Functions**:
        *   `create_capsule`: Adds a new capsule to the system.
        *   `update_capsule`: Modifies an existing capsule.
        *   `delete_capsule`: Removes a capsule from the system.
        *   `list_capsules`: Lists all capsules in the system.
        *   `capsule_detail`: Retrieves detailed information about a specific capsule.
*   **`contributors.rs`**:
    
    *   **Purpose**: Manages contributors who own or create capsules. It handles CRUD operations for contributors and relates them to the capsules they contribute to.
    *   **Key Functions**:
        *   `create_contributor`: Registers a new contributor.
        *   `update_contributor`: Updates existing contributor details.
        *   `delete_contributor`: Removes a contributor from the system.
        *   `list_contributors`: Lists all contributors.
        *   `get_contributor_with_capsules`: Retrieves a specific contributor along with the capsules they are associated with.
*   **`items.rs`**:
    
    *   **Purpose**: Manages the items stored within capsules, such as documents, images, or other digital content. This file contains the logic for CRUD operations on items.
    *   **Key Functions**:
        *   `add_item_to_capsule`: Adds a new item to a specific capsule.
        *   `delete_item_from_capsule`: Removes an item from a capsule.
        *   `update_item_details`: Updates the details of an existing item within a capsule.
        *   `get_all_items`: Retrieves all items across all capsules.
        *   `get_item`: Gets a specific item by its ID.
*   **`merges.rs`**:
    
    *   **Purpose**: Handles the merging of two capsules into one, which is a complex operation that involves transferring all items from one capsule to another and ensuring that all references are updated accordingly.
    *   **Key Functions**:
        *   `merge_capsules`: Merges two specified capsules into one.
        *   `get_merges`: Lists all the capsule merges that have occurred.
*   **`main.rs`**:
    
    *   **Purpose**: The entry point of the Rust application, setting up the Rocket framework, routing, and state management. It initializes and mounts all the routes and manages shared state across the application.
    *   **Initialization**: Loads initial data from JSON files into the system and sets up the web server with routes from other modules.
*   **Data Directory**:
    
    *   Contains example JSON files for `capsules.json`, `contributors.json`, and `items.json` which are used to pre-load data into the application on startup.

