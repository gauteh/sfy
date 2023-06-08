# Step by Step - CurrentRanger instructions

### Installation (linux)

1. Download the files from GitHub using the command:

		git clone https://github.com/MGX3D/CurrentViewer

2. Go to the directory "CurrentViewer"

3. Add any missing dependencies from requirements.txt to the conda environment.


### How to use CurrentViewer

Make sure you are in the "CurrentViewer" directory

For streaming option use the command:

		python3 current_viewer.py -p /dev/tty/ACM0

To create and save a CVS file use the command:

		python current_viewer.py -p /dev/ttyACM0 -g --out filename.cvs


Note:

- Make sure you use the correct USB or ACM port in the commnad
- For more storing options go to https://github.com/MGX3D/CurrentViewer


### How to use CurrantRanger device

- To turn on the device you press the "on" button one time. A light will come on. To turn the device off you hold the "on" button in for 2 seconds.

- When the device is turned on it goes automatically into "manual mode", meaning you can use the pads to switch from nA to µA to mA by tuching the golden plates.

- To put on the "Bias mode" you tap "µA" and "mA" once at the same time. To turn of you repeat the process. (The "Bias mode" allows for positive output terminal to swing both positively and negatively allowing for AC current measurments)

- To put on the "LPF filter" (low-Pass filter), tap "nA" and "µA" once at the same time. To turn off the filter you repeat the process. The filter removes a lot of high frequency noise, making the curve smoother.

- To go into "Auto-ranging mode", tap "nA" and "mA" once at the same time. In this mode you can no longer switch between the difference ranges by tapping the golden pads. To go back to "manuel mode" repeat the process.

- If non of the gold pannels are touched within 10min it will automatically switch off. To avoid it turning off, touch one of golden pads. The device will make a beeping sound before it turns off.


