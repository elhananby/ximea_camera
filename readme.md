### XIMEA PCIE Camera
While Python was enought to capture and save 800x600 Mono8 frames at 500fps, using the new XIMEA highspeed, high resolution camera (4K at 400fps) required migrating the code to a faster, more efficient language.

#### Basic algorithm
The basic idea of this algorithm is that we want don't know when a fly might enter the trigger zone, so optimally we want to save a few frames before the fly is actually visible. To do so, we save all incoming frames into a circular buffer with length (FPS * Time Before). Whenever this buffer is filled, it pops an item from the start, and adds the new frame to the end.
When a trigger arrives, we switch to another buffer with length (FPS * Time After). When that buffer is full, the data is concatenated and saved to disk as tiffs.
These tiffs can either be immediately passed to ffmpeg for video writing, or just saved for later processing.

The project should be easily compiled using `cargo build --release`, and is already integrated into the main BraidTrigger Python scripts.
