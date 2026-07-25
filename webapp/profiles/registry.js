import { profile as aj179 } from "./aj179.js";
import { profile as aj179apex } from "./aj179apex.js";
import { profile as aj159apex } from "./aj159apex.js";
import { profile as aj159pro } from "./aj159pro.js";
import { profile as aj159 } from "./aj159.js";
import { profile as aj159mc } from "./aj159mc.js";
import { profile as aj139pro } from "./aj139pro.js";
import { profile as attackshark_r1 } from "./attackshark_r1.js";
import { profile as attackshark_x3 } from "./attackshark_x3.js";
import { profile as vxe_f1 } from "./vxe_f1.js";

export const PROFILES = [
  aj179,
  aj179apex,
  aj139pro,
  aj159,
  aj159apex,
  aj159pro,
  aj159mc,
  attackshark_r1,
  attackshark_x3,
  vxe_f1,
];

class ProfileRegistry {
  constructor() {
    this.profiles = PROFILES;
  }

  getAllProfiles() {
    return this.profiles;
  }

  getProfile(id) {
    return this.profiles.find((p) => p.id === id) || this.profiles[0];
  }

  findProfile(vid, pid) {
    return this.findProfileByHid(vid, pid);
  }

  findProfileByHid(vid, pid) {
    const vidHex = typeof vid === "number" ? vid.toString(16).toUpperCase().padStart(4, "0") : String(vid).replace("0x", "").toUpperCase().padStart(4, "0");
    const pidHex = typeof pid === "number" ? pid.toString(16).toUpperCase().padStart(4, "0") : String(pid).replace("0x", "").toUpperCase().padStart(4, "0");

    for (const p of this.profiles) {
      for (const m of p.modes) {
        const mVid = m.vid.replace("0x", "").toUpperCase().padStart(4, "0");
        const mPid = m.pid.replace("0x", "").toUpperCase().padStart(4, "0");
        if (mVid === vidHex && mPid === pidHex) {
          return p;
        }
      }
    }
    return null;
  }
}

export const profileRegistry = new ProfileRegistry();
