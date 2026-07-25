import { profile as r1 } from "./attackshark_r1.js";

/**
 * Attack Shark X11 uses the same reported BK3633 / CX5285-compatible HID
 * family as the R1. Its receiver VID:PID is intentionally listed separately
 * so a status-reported device ID can refine the selected model after connect.
 */
export const profile = {
  ...r1,
  id: "attackshark_x11",
  name: "Attack Shark X11",
  modes: [
    { value: 0, desc: "2.4G Wireless", vid: "1D57", pid: "FA60", devId: "AS_X11" },
    { value: 1, desc: "Wired USB", vid: "1D57", pid: "FA55", devId: "AS_X11" },
  ],
};
