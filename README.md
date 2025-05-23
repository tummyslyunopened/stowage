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
- CORS support
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

## API Endpoints

### Upload a File

```
POST /upload
Content-Type: multipart/form-data
```

**Example Response:**
```json
{
  "file_id": "550e8400-e29b-41d4-a716-446655440000",
  "download_url": "/files/550e8400-e29b-41d4-a716-446655440000"
}
```

### Download a File

```
GET /files/{file_id}
```

## Configuration

Environment variables:

- `HOST`: Server host (default: 0.0.0.0)
- `PORT`: Server port (default: 8080)
- `MEDIA_PATH`: Path to store uploaded files (default: /app/media)
- `MAX_FILE_SIZE`: Maximum file size in bytes (default: 100MB)

## License

MIT
