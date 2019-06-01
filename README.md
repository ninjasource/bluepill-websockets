# bluepill-websockets
An example websocket application using the stm32 bluepill and a w5500 ethernet board

How to build:
A simple "cargo build" will work. This is equivalent to "cargo build --features="stm32f103,rt" as it seems to pick the required features up somehow. This builds on rust stable.
When the binary gets too large you have to build release "cargo build --release" so that it fits in 64KB as defined in the memory.x file

How to upload to bluepill using IntelliJ IDEA:
Open a command window in this folder and enter "openocd". This will start an openocd server with setting specified in the openocd.cfg file.
A simple "cargo run" will then execute the openocd.gdb script to upload the binary to the bluepill and break at the main function.
You need to enter "continue" to continue running the application.

How to upload to bluepill using Visual Studio Code:
Make sure the binary has been built in release mode and that the openocd server (as described above) is NOT running.
Visual studio code uses the Coretex-Debug extension to upload and begin a gdb session. You can put breakpoints in the code and step through code.
This is all setup with the launch.json file in the .vscode folder. The openocd.cfg and openocd.gdb are not used

How to see log messages:
Logs are done through ITM (Instrumentation Trace Macrocell) which is very fast but needs a serial to usb adapter.
Use the tool serialitm. There is a run.bat file for ease of use. This is setup for BAUD 1000000 and COM3.
Connect the serial USB to UART device to the computer. Should come up in device manager on one of the com ports (e.g. Com3)
Connect ground to ground and Rx to pin B3. This is the ITM stimulus pin on the bluepill.