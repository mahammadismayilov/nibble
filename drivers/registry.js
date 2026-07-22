import { compxDriver } from "./compx.js";
import { yichipDriver } from "./yichip.js";

/**
 * Registry of registered micro-controller protocol drivers.
 */
class DriverRegistry {
  constructor() {
    this.drivers = new Map();
    this.register(compxDriver);
    this.register(yichipDriver);
  }

  register(driver) {
    this.drivers.set(driver.id, driver);
  }

  getDriver(driverId) {
    return this.drivers.get(driverId) || compxDriver;
  }

  getDriverForVid(vid) {
    const num = typeof vid === "string" ? parseInt(vid.replace("0x", ""), 16) : vid;
    for (const driver of this.drivers.values()) {
      if (driver.supportsVendor(num)) return driver;
    }
    return compxDriver;
  }
}

export const driverRegistry = new DriverRegistry();
