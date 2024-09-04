# Vibe API Server

## Overview

Vibe API Server is a spinoff of the Vibe desktop application, focusing on providing a robust API for audio/video transcription services. This server-side application leverages the core functionality of Vibe while offering a more flexible, scalable solution for integration into various workflows and applications.

## Features

-   support running as a standalone API server on macOS with silicon CPU(M1 series and above) and debian 12 (vulkan supported)
-   RESTful API for audio/video transcription
-   Support for multiple languages and speaker diarization
-   File upload and remote file processing capabilities
-   Swagger UI for easy API exploration and testing
-   Configurable transcription options

## API Endpoints

1. `/transcribe` (POST): Transcribe uploaded audio/video files
2. `/load` (POST): Load a specific transcription model
3. `/list` (GET): List available transcription models

## Getting Started

### Prerequisites

-   Rust toolchain
-   Cargo

### Installation

> TBD

## Configuration

-   Server host and port can be configured in `config.toml`
-   Transcription models should be placed in the `models` directory

## Usage

1. Start the server
2. Access the Swagger UI at `http://localhost:3000/docs` for API documentation and testing
3. Use the API endpoints in your application

## Development

### Guidelines

-   always use wisely docstrings for later documentation and readability
-   focus on providing production-ready code
-   be careful with terminology, especially when it comes to audio/video processing and transcription

### Remove desktop dependencies completely

### Adding New Features

1. Enhance `src/server.rs` for new API endpoints
2. Update `src/main_server.rs` for any initialization logic
3. Modify `Cargo.toml` to add new dependencies or features

### Keeping Up with Upstream Changes

1. Regularly merge changes from the main Vibe repository
2. Resolve conflicts, ensuring API enhancements don't break core functionality

## Future Enhancements

-   [ ] Authentication and rate limiting
-   [ ] Asynchronous processing for large files
-   [ ] WebSocket support for real-time transcription updates
-   [ ] Docker containerization for easy deployment

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting pull requests.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
