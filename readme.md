# Camera Control and Image Processing

This Rust project provides a robust system for camera control, image acquisition, and processing. It's designed to work with specific camera hardware (likely using the XIMEA API) and includes features for real-time image capture, processing, and saving based on external triggers.

## Features

- Camera control and parameter setting
- Real-time image acquisition
- Integration with ZeroMQ for message-based triggering
- Kalman filter-based object tracking
- Dynamic video saving based on triggers
- Command-line interface for easy configuration

## Prerequisites

- Rust (latest stable version)
- XIMEA camera and SDK
- FFmpeg (for video encoding)
- ZeroMQ library

## Installation

1. Clone the repository:
   ```
   git clone [your-repo-url]
   cd [your-repo-name]
   ```

2. Build the project:
   ```
   cargo build --release
   ```

## Usage

Run the application with the following command:

```
cargo run --release -- [OPTIONS]
```

### Command-line Options

- `--serial`: Camera serial number (default: 0)
- `--fps`: Frames per second (default: 500.0)
- `--exposure`: Exposure time in microseconds (default: 2000.0)
- `--width`: Image width (default: 2016)
- `--height`: Image height (default: 2016)
- `--offset-x`: X offset (default: 1056)
- `--offset-y`: Y offset (default: 170)
- `--t-before`: Time to record before trigger in seconds (default: 0.5)
- `--t-after`: Time to record after trigger in seconds (default: 1.0)
- `--address`: ZeroMQ server address (default: "127.0.0.1")
- `--sub-port`: ZeroMQ subscriber port (default: "5556")
- `--req-port`: ZeroMQ request port (default: "5557")
- `--debug`: Enable debug mode (flag)
- `--save-folder`: Folder to save output (default: "None")

## How It Works

This program operates as a continuous image acquisition and processing system with event-driven video saving. Here's a breakdown of its operation:

1. **Initialization**: 
   - The program starts by parsing command-line arguments to set up camera parameters and operational settings.
   - It initializes the XIMEA camera with the specified settings (resolution, framerate, exposure, etc.).
   - A ZeroMQ subscriber is set up to listen for external triggers.

2. **Image Acquisition**:
   - The camera continuously captures images at the specified framerate.
   - Each captured frame is wrapped in an `ImageData` struct containing the image data and metadata.

3. **Message Handling**:
   - Concurrently, the program listens for ZeroMQ messages.
   - Messages are expected to contain JSON data with Kalman filter estimates for object tracking.

4. **Frame Buffering**:
   - Captured frames are continuously buffered in memory.
   - The buffer size is determined by the `--t-before` and `--t-after` parameters.

5. **Trigger Processing**:
   - When a valid trigger message is received, it initiates the video saving process.
   - The program collects frames from before the trigger (based on `--t-before`) and continues capturing for the duration specified by `--t-after`.

6. **Video Saving**:
   - The collected frames are passed to a separate thread for processing and saving.
   - FFmpeg is used to encode the frames into an MP4 video file.
   - Video metadata is saved alongside the video file.

7. **Continuous Operation**:
   - The program continues this cycle of capturing, buffering, and saving until a "kill" signal is received.

## Detailed Usage Instructions

1. **Setting Up the Camera**:
   - Ensure your XIMEA camera is connected and recognized by the system.
   - Use the `--serial` option if you have multiple cameras and need to specify a particular one.
   - Adjust `--width`, `--height`, `--offset-x`, and `--offset-y` to set the region of interest on the sensor.

2. **Configuring Acquisition Parameters**:
   - Set the desired framerate with `--fps`. Note that this affects the maximum exposure time.
   - Adjust the exposure time with `--exposure`. This is in microseconds.

3. **Setting Up ZeroMQ Communication**:
   - Ensure your ZeroMQ server is running at the specified address and ports.
   - Use `--address`, `--sub-port`, and `--req-port` to configure the ZeroMQ connection.

4. **Configuring Video Saving**:
   - Set `--t-before` and `--t-after` to control how much video is saved around each trigger event.
   - Specify the output directory with `--save-folder`.

5. **Running the Program**:
   - Start the program with your desired configuration.
   - The program will output log messages indicating its status and any received triggers.
   - Videos will be automatically saved to the specified folder when triggers are received.

6. **Monitoring and Debugging**:
   - Use the `--debug` flag to enable more verbose logging if you need to troubleshoot issues.

7. **Shutting Down**:
   - The program will run continuously until it receives a "kill" message through ZeroMQ.
   - Ensure you send this message to gracefully shut down the program and ensure all data is saved.

## Integration with External Systems

This program is designed to work as part of a larger system:

- It expects to receive trigger messages and Kalman filter data through ZeroMQ.
- The trigger messages should contain JSON-formatted data with object tracking information.
- Ensure your object tracking system is configured to send data in the expected format (see `KalmanEstimateRow` struct in `structs.rs`).

By following these instructions and understanding the program's workflow, you can effectively use this camera control and image processing system in your scientific or industrial applications.

## Project Structure

- `main.rs`: Entry point of the application
- `camera.rs`: Camera control and parameter setting
- `cli.rs`: Command-line interface parsing
- `frames.rs`: Frame handling and video saving
- `helpers.rs`: Utility functions
- `messages.rs`: ZeroMQ message handling
- `structs.rs`: Data structures used throughout the project