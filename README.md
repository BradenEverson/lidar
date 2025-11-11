# RPLiDAR Communication Implementation and Tiny Visualization Dashboard for the Pi
This repo implements the legacy and (currently unfinished) ultra cabin scan packet protocols for the RPLiDAR A1. It sets up convenience methods for hooking callbacks or mpsc channels for consuming scans as they come in. The project itself creates a minimal webserver that streams LiDAR scans over websocket. It is not the cleanest, I just wanted something to visualize the LiDAR in real time :)

<img width="662" height="642" alt="image" src="https://github.com/user-attachments/assets/5a136040-6733-4df4-bc38-30d5c4cba0e3" />
