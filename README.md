# asi-rs
ZWO ASI multiplatform drivers written in Rust

## Features (and relative status)

CCD:
 - Get count of connected cameras--> ASIGetNumOfConnectedCameras | **IMPLEMENTED**
 - Get cameras' ID and other informations--> ASIGetCameraProperty | **IMPLEMENTED**
 - Open camera -->ASIOpenCamera | **IMPLEMENTED**
 - Initialize-->ASIInitCamera | **IMPLEMENTED**
 - Get count of control type--> ASIGetNumOfControls | **IMPLEMENTED**
 - Get capacity of every control type-->ASIGetControlCaps | **IMPLEMENTED**
 - Set image size and format-->ASISetROIFormat | **NOT IMPLEMENTED**
 - Set start position when ROI-->ASISetStartPos | **NOT IMPLEMENTED**
 - Get control value-->ASIGetControlValue | **IMPLEMENTED**
 - Set control value-->ASISetControlValue | **IMPLEMENTED**
 - Start video capture-->ASIStartVideoCapture | **NOT IMPLEMENTED**
 - Stop video capture-->ASIStopVideoCapture | **NOT IMPLEMENTED**
 - Get video frames-->ASIGetVideoData | **NOT IMPLEMENTED**
 - Start image exposure-->ASIStartExposure | **IMPLEMENTED**
 - Cancel exposure-->ASIStopExposure | **IMPLEMENTED**
 - Get snap status-->ASIGetExpStatus | **IMPLEMENTED**
 - Close camera-->ASICloseCamera | **IMPLEMENTED**
 - Get supported mode of the camera--> ASIGetCameraSupportMode | **NOT IMPLEMENTED**
 - Set a mode --> ASISetCameraMode | **NOT IMPLEMENTED**
 - Get the mode--> ASIGetCameraMode | **NOT IMPLEMENTED**
 - Send a trigger signal for software simulation-->ASISendSoftTrigger | **NOT IMPLEMENTED**
 - Get version string of SDK-->ASIGetSDKVersion | **NOT IMPLEMENTED**
 - Send ST4 guiding pulse start guiding-->ASIPulseGuideOn | **NOT IMPLEMENTED**
 - Send ST4 guiding pulse stop guiding-->ASIPulseGuideOff | **NOT IMPLEMENTED**
