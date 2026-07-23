import { state, saveState, device, profile } from "../state.js";
import { KEY_LABELS, FUNC_GROUPS, FUNC_LABEL } from "../constants.js";
import { KEY_FUNC_PROVEN } from "../../protocol.js";
import { queueDeviceWrite, flushDeviceWrites } from "../writer.js";
import { toast } from "../state.js";

export function renderKeys() {
  const d = device();
  const p = profile();
  const host = document.getElementById("key-hotspots");
  if (!host) return;
  host.innerHTML = "";
  d.keys.forEach((k) => {
    const func = p.keys[k.id] || k.defaultFunc;
    const div = document.createElement("div");
    div.className = `hotspot ${k.direction || "left"}${
      state.selectedKey === k.id ? " active" : ""
    }`;
    div.style.left = `${k.x}%`;
    div.style.top = `${k.y}%`;
    div.innerHTML = `<span class="pin"></span><span class="label">${KEY_LABELS[k.id]} · ${
      FUNC_LABEL[func] || func
    }</span>`;
    div.addEventListener("click", () => {
      state.selectedKey = k.id;
      renderKeys();
    });
    host.appendChild(div);
  });

  const keyMeta = d.keys.find((k) => k.id === state.selectedKey) || d.keys[0];
  const current = p.keys[keyMeta.id] || keyMeta.defaultFunc;
  const labelEl = document.getElementById("selected-key-label");
  if (labelEl) {
    labelEl.textContent = `${KEY_LABELS[keyMeta.id]} → ${FUNC_LABEL[current] || current}`;
  }

  const groups = document.getElementById("func-groups");
  if (!groups) return;
  groups.innerHTML = "";
  FUNC_GROUPS.forEach((g) => {
    const wrap = document.createElement("div");
    wrap.className = "func-group";
    wrap.innerHTML = `<h4>${g.title}</h4>`;
    const opts = document.createElement("div");
    opts.className = "func-options";
    g.items.forEach((item) => {
      const btn = document.createElement("button");
      btn.type = "button";
      const proven = KEY_FUNC_PROVEN.has(item.id);
      btn.textContent = item.label;
      btn.title = proven ? item.label : "Not available yet";
      if (!proven) btn.classList.add("unproven");
      if (item.id === current) btn.classList.add("active");
      btn.addEventListener("click", () => assignKey(keyMeta, item.id));
      opts.appendChild(btn);
    });
    wrap.appendChild(opts);
    groups.appendChild(wrap);
  });
}

export function assignKey(keyMeta, funcId) {
  const p = profile();
  if (keyMeta.defaultFunc === "left" && p.settings.lmbLock && funcId !== "left") {
    toast("Left-click is locked. Unlock it in Settings first.");
    return;
  }
  if (keyMeta.defaultFunc === "left" && funcId !== "left") {
    if (!confirm("Changing left-click may make UI hard to use. Continue?")) return;
  }
  if (!KEY_FUNC_PROVEN.has(funcId)) {
    toast("That function isn’t available yet — pick another");
    return;
  }
  p.keys[keyMeta.id] = funcId;
  saveState();
  renderKeys();
  queueDeviceWrite("keys");
}

export async function applyKeysOnly() {
  queueDeviceWrite("keys");
  await flushDeviceWrites();
}
