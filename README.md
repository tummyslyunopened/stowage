# Stowage - File Server

A high-performance file server for audio, video, images, RSS, and JSON files, built with Rust and Actix-web.

## Features

- RESTful API for file uploads
- Unique, non-sequential file IDs
- Optimized for serving various file types:
  - Audio files (mp3, wav, ogg, etc.)
  - Video files (mp4, webm, mov, etc.)
  - Images (jpg, png, gif, etc.)
  - RSS/XML feeds
  - JSON files
- Built with Docker for easy deployment
- File type validation
- Configurable file size limits

## Prerequisites

- Docker and Docker Compose
- Rust toolchain (if building locally)

## Getting Started

### Using Docker (Recommended)


1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd stowage
   ```

2. Build and run the Docker container:
   ```bash
   docker-compose up --build
   ```

### Local Development

1. Install Rust from https://rustup.rs/

2. Clone the repository:
   ```bash
   git clone <repository-url>
   cd stowage
   ```

3. Build and run the application:
   ```bash
   cargo run --release
   ```


## Comprehensive API Documentation

Stowage is a high-performance file server for audio, video, images, RSS, and JSON files, built with Rust and Actix-web.

### Endpoints

#### 1. `POST /upload`

**Description:**  
Upload a file to the server. The file will be validated for allowed types and stored with a unique, non-sequential ID.

**Request:**
- Content-Type: `multipart/form-data`
- Form field: `file` (the file to upload)

**Response (201 Created):**
```json
{
  "file_id": "e.g. 123e4567-e89b-12d3-a456-426614174000",
  "download_url": "/files/123e4567-e89b-12d3-a456-426614174000"
}
```

**Errors:**
- 400 Bad Request: Invalid file type, missing file, or upload error.

---

#### 2. `GET /files/{file_id}`

**Description:**  
Download a previously uploaded file by its unique ID.

**Request:**
- Path parameter: `file_id` (the UUID returned from upload)

**Response (200 OK):**
- The file as a binary stream, with appropriate content-type.

**Errors:**
- 404 Not Found: File does not exist.

---

#### 3. `GET /about`

**Description:**  
Get information about the Stowage server.

**Response (200 OK):**
- Plain text description of the server and its features.

---

### Allowed File Types

- Audio, video, image files (by MIME type)
- JSON (`application/json`)
- XML (`application/xml`, `text/xml`, `application/rss+xml`)
- Octet-stream (`application/octet-stream`)

Files are validated by both extension and content.

---

### Example Python Usage

#### Upload a File

```python
import requests

url = "http://localhost:8080/upload"
file_path = "example.json"  # Replace with your file

with open(file_path, "rb") as f:
    files = {"file": (file_path, f)}
    response = requests.post(url, files=files)

print("Status:", response.status_code)
print("Response:", response.json())
```

#### Download a File

```python
import requests

file_id = "your-file-id-here"  # Replace with the file_id from upload response
url = f"http://localhost:8080/files/{file_id}"

response = requests.get(url)
if response.status_code == 200:
    with open("downloaded_file", "wb") as f:
        f.write(response.content)
    print("File downloaded successfully.")
else:
    print("Download failed:", response.status_code)
```

#### Get About Information

```python
import requests

url = "http://localhost:8080/about"
response = requests.get(url)
print(response.text)
```

---

### Notes

- The server stores files in a directory specified by the `MEDIA_PATH` environment variable (default: `/app/media`).
- File IDs are UUIDs and do not reveal upload order or file type.
- Only allowed file types are accepted; others are rejected with a 400 error.

## Configuration

Environment variables:

- `HOST`: Server host (default: 0.0.0.0)
- `PORT`: Server port (default: 8080)
- `MEDIA_PATH`: Path to store uploaded files (default: /app/media)
- `MAX_FILE_SIZE`: Maximum file size in bytes (default: 100MB)

## License

MIT
