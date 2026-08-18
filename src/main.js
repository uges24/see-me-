import "./style.css";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { clockAngles } from "./clock.js";

const nativeRuntime = isTauri();
const objectWindow = new URLSearchParams(location.search).get("window");
const clockWindow = objectWindow === "clock";
const photoWindow = objectWindow === "photo";

const faces = [
  { id: "koi", name: "Koi Nocturne", type: "Koi", tone: "indigo", free: true, note: "Original ornamental dial" },
  { id: "orbit", name: "Aurora Orrery", type: "Orbit", tone: "teal", free: true, note: "Layered celestial metalwork" },
  { id: "flower", name: "Verdant Halo", type: "Flower", tone: "lime", free: true, note: "Translucent cast resin" },
  { id: "amber", name: "Tangerine Tide", type: "Fish", tone: "amber", free: true, note: "Marbled artisan resin" }
];

const defaults = {
  face: "koi",
  showSeconds: true,
  smooth: true,
  animate: true,
  alwaysOnTop: true,
  opacity: 1,
  size: 360,
  mode: "smooth",
  photo: "",
  photoScale: 1,
  photoX: 50,
  photoY: 50,
  photoObject: null,
  enabled: true,
  x: 80,
  y: 80,
  monitor: null,
  scaleFactor: 1,
  locked: true,
  visibilityBehaviour: "hide",
  clickThrough: false,
  ghostHideDelay: 0,
  ghostReturnDelay: 150,
  fadeOpacity: .15,
  visible: true,
  launchAtLogin: false
};

let saved = {};
try { saved = JSON.parse(localStorage.getItem("timepiece-studio") || "{}"); } catch { localStorage.removeItem("timepiece-studio"); }
if (saved.behaviour && !saved.visibilityBehaviour) {
  saved.visibilityBehaviour = saved.behaviour === "ghost" ? "hide" : saved.behaviour === "stay" ? "do-nothing" : saved.behaviour;
  saved.clickThrough = saved.behaviour === "click-through";
}
if (!faces.some((face) => face.id === saved.face)) saved.face = defaults.face;
const state = { ...defaults, ...saved, screen: "home", inspector: null, addOpen: false, widgetOpen: true };

const icon = (name, size = 18) => {
  const paths = {
    grid: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>',
    settings: '<path d="M12 15.2a3.2 3.2 0 1 0 0-6.4 3.2 3.2 0 0 0 0 6.4Z"/><path d="m19.4 15 .2 1.7-2 1.2-1.4-1a7.6 7.6 0 0 1-1.5.9l-.4 1.7h-4.6l-.4-1.7a7.6 7.6 0 0 1-1.5-.9l-1.4 1-2-1.2.2-1.7a7.4 7.4 0 0 1 0-1.8l-1.7-.8V10l1.7-.8a7.4 7.4 0 0 1 0-1.8l-.2-1.7 2-1.2 1.4 1a7.6 7.6 0 0 1 1.5-.9l.4-1.7h4.6l.4 1.7a7.6 7.6 0 0 1 1.5.9l1.4-1 2 1.2-.2 1.7a7.4 7.4 0 0 1 0 1.8l1.7.8v2.4l-1.7.8a7.4 7.4 0 0 1 0 1.8Z"/>',
    gallery: '<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="m21 15-4.5-4.5L6 21"/>',
    upload: '<path d="M12 16V4"/><path d="m7 9 5-5 5 5"/><path d="M4 20h16"/>',
    close: '<path d="m6 6 12 12M18 6 6 18"/>',
    expand: '<path d="M8 3H3v5M16 3h5v5M21 16v5h-5M3 16v5h5"/>',
    move: '<path d="M12 3v18M3 12h18"/><path d="m8 7 4-4 4 4M8 17l4 4 4-4M7 8l-4 4 4 4M17 8l4 4-4 4"/>',
    spark: '<path d="m12 2 1.6 6.4L20 10l-6.4 1.6L12 18l-1.6-6.4L4 10l6.4-1.6L12 2Z"/>'
  };
  return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name]}</svg>`;
};

const runtimeSettings = () => ({
  selectedFace: state.face,
  x: state.x,
  y: state.y,
  width: state.size,
  height: state.size,
  monitor: state.monitor,
  scaleFactor: state.scaleFactor,
  alwaysOnTop: state.alwaysOnTop,
  locked: state.locked,
  visibilityBehaviour: state.visibilityBehaviour,
  clickThrough: state.clickThrough,
  ghostHideDelay: state.ghostHideDelay,
  ghostReturnDelay: state.ghostReturnDelay,
  fadeOpacity: state.fadeOpacity,
  enabled: state.enabled,
  showSecondHand: state.showSeconds,
  smoothMovement: state.smooth,
  visible: state.visible,
  launchAtLogin: state.launchAtLogin
});

const persist = () => {
  const { screen, inspector, addOpen, widgetOpen, photoObject, photoPreview, behaviour, ...stored } = state;
  localStorage.setItem("timepiece-studio", JSON.stringify(stored));
  if (nativeRuntime && !objectWindow) invoke("update_settings", { settings: runtimeSettings() }).catch(console.error);
};

function faceMarkup(face, compact = false) {
  const sizeClass = compact ? "dial--compact" : "";
  const art = {
    Koi: `<div class="ornate-ring"></div><div class="wave wave--one"></div><div class="wave wave--two"></div><div class="koi koi--one"></div><div class="koi koi--two"></div><div class="diamond diamond--one"></div><div class="diamond diamond--two"></div>`,
    Orbit: `<div class="orbit-core"></div><div class="orbit orbit--one"></div><div class="orbit orbit--two"></div><i class="planet planet--one"></i><i class="planet planet--two"></i><i class="planet planet--three"></i>`,
    Flower: `<div class="petal petal--a"></div><div class="petal petal--b"></div><div class="petal petal--c"></div><div class="petal petal--d"></div><div class="petal petal--e"></div><div class="flower-core"></div>`,
    Fish: `<div class="amber-vein vein--one"></div><div class="amber-vein vein--two"></div><div class="amber-vein vein--three"></div><div class="amber-speck speck--one"></div><div class="amber-speck speck--two"></div>`
  }[face.type];
  return `<div class="dial dial--${face.id} ${sizeClass}" data-face="${face.id}" aria-label="${face.name} live analogue clock"><div class="dial-art">${art}</div><div class="numerals"><b>12</b><b>3</b><b>6</b><b>9</b></div><div class="ticks"></div><div class="hands"><i class="hand hand--hour"></i><i class="hand hand--minute"></i><i class="hand hand--second ${state.showSeconds ? "" : "is-hidden"}"></i><span class="hub"></span></div></div>`;
}

function appMarkup() {
  const active = faces.find((face) => face.id === state.face) || faces[0];
  const titles = { home: "What’s on your screen.", objects: "Your desktop objects.", gallery: "Choose the atmosphere.", settings: "App settings." };
  return `<main class="shell">
    <aside class="rail" aria-label="Primary navigation">
      <div class="brand">T<span>/</span></div>
      <nav>
        <button class="rail-button ${state.screen === "home" ? "is-active" : ""}" data-screen="home" aria-label="Home">${icon("spark")}</button>
        <button class="rail-button ${state.screen === "objects" ? "is-active" : ""}" data-screen="objects" aria-label="Objects">${icon("grid")}</button>
        <button class="rail-button ${state.screen === "gallery" ? "is-active" : ""}" data-screen="gallery" aria-label="Clock Faces">${icon("gallery")}</button>
        <button class="rail-button ${state.screen === "settings" ? "is-active" : ""}" data-screen="settings" aria-label="Settings">${icon("settings")}</button>
      </nav>
      <button class="rail-button rail-button--bottom" id="addObject" aria-label="Add object">${icon("upload")}</button>
    </aside>
    <section class="workspace">
      <header class="topbar"><div><p class="eyebrow">TIMEPIECE STUDIO</p><h1>${titles[state.screen] || titles.home}</h1></div><div class="topbar-meta"><span class="live-dot"></span><span>LOCAL ONLY</span></div></header>
      ${state.screen === "home" ? homeMarkup(active) : state.screen === "objects" ? objectsMarkup(active) : state.screen === "gallery" ? galleryMarkup() : settingsMarkup()}
    </section>
    ${state.addOpen ? addObjectMarkup() : ""}
  </main>`;
}

function objectCard(type, active, compact = false) {
  const isClock = type === "clock";
  const object = isClock ? state : state.photoObject;
  if (!object?.enabled) return "";
  const art = isClock ? faceMarkup(active, true) : state.photoPreview ? `<img class="object-photo-preview" src="${state.photoPreview}" alt="Your photo" />` : `<div class="photo-missing">PHOTO</div>`;
  const name = isClock ? active.name : "Photo";
  return `<article class="object-card ${compact ? "object-card--compact" : ""}"><div class="object-card-art">${art}</div><div class="object-card-copy"><p class="eyebrow">${isClock ? "CLOCK" : "PHOTO"}</p><h3>${name}</h3><span>${object.visible ? "Visible" : "Hidden"} · ${object.alwaysOnTop ? "Always on top" : "Normal layer"}</span></div><div class="object-card-actions"><button class="button button--light" data-edit-object="${type}">Edit</button><button class="button button--outline" data-toggle-object="${type}">${object.visible ? "Hide" : "Show"}</button><button class="text-action" data-remove-object="${type}">Remove</button></div></article>`;
}

function homeMarkup(active) {
  const cards = [objectCard("clock", active, true), objectCard("photo", active, true)].filter(Boolean).join("");
  return `<section class="home-view"><div class="home-lead"><div><p class="eyebrow">ACTIVE OBJECTS</p><h2>${cards ? "Present, never in the way." : "Your desktop is clear."}</h2><p>${cards ? "See what is floating now. Edit any object in one step." : "Add a clock or photo to begin."}</p></div><button class="button button--light add-object-primary" id="addObjectPrimary">+ Add object</button></div><div class="object-grid">${cards || `<button class="empty-object" id="emptyAdd">+ Add your first object</button>`}</div></section>`;
}

function objectsMarkup(active) {
  if (state.inspector) return inspectorMarkup(state.inspector, active);
  const cards = [objectCard("clock", active), objectCard("photo", active)].filter(Boolean).join("");
  return `<section class="objects-view"><div class="section-bar"><p>${cards ? "Everything currently placed on your desktop." : "No active objects."}</p><button class="button button--light" id="addObjectSecondary">+ Add object</button></div><div class="object-list">${cards || `<button class="empty-object" id="emptyAdd">+ Add object</button>`}</div></section>`;
}

function inspectorMarkup(type, active) {
  const isClock = type === "clock";
  const object = isClock ? state : state.photoObject;
  if (!object) return `<section class="inspector"><button class="back-action" id="closeInspector">← Objects</button><p>That object is no longer available.</p></section>`;
  return `<section class="inspector"><button class="back-action" id="closeInspector">← Objects</button><div class="inspector-head"><div><p class="eyebrow">${isClock ? "CLOCK" : "PHOTO"} INSPECTOR</p><h2>${isClock ? active.name : "Your photo"}</h2><p>Choose how this object behaves. Changes apply immediately.</p></div><div class="inspector-preview">${isClock ? faceMarkup(active, true) : state.photoPreview ? `<img class="object-photo-preview" src="${state.photoPreview}" alt="Your photo" />` : `<div class="photo-missing">PHOTO</div>`}</div></div><div class="inspector-controls">${!isClock ? `<button class="setting-row" id="changePhoto"><span>Change photo</span><b>CHOOSE FILE</b></button>` : ""}<label class="select-row"><span>On hover</span><select id="objectHover"><option value="hide">Hide</option><option value="fade">Fade</option><option value="do-nothing">Do nothing</option></select></label>${switchRow("objectClicks", "Let clicks pass through", object.clickThrough)}${switchRow("objectTop", "Always on top", object.alwaysOnTop)}${switchRow("objectLock", "Lock position", object.locked)}${switchRow("objectVisible", "Show object", object.visible)}<label class="range-row"><span>Size <b>${object.width || state.size}px</b></span><input id="objectSize" type="range" min="180" max="720" value="${object.width || state.size}" /></label>${isClock ? `${switchRow("secondsToggle", "Second hand", state.showSeconds)}${switchRow("smoothToggle", "Smooth movement", state.smooth)}<button class="setting-row" data-screen="gallery"><span>Change face</span><b>${active.name}</b></button>` : ""}<button class="danger-action" id="removeInspected">Remove ${isClock ? "clock" : "photo"}</button></div></section>`;
}

function switchRow(id, label, enabled) {
  return `<button class="setting-row" id="${id}"><span>${label}</span><b>${enabled ? "ON" : "OFF"}</b></button>`;
}

function galleryMarkup() {
  return `<section class="gallery-view"><div class="gallery-intro"><p>Four original faces. Choose one and the floating clock updates immediately.</p></div><div class="face-grid">${faces.map((face, index) => `<article class="face-card ${state.face === face.id ? "is-selected" : ""}" style="--delay:${index * 65}ms"><div class="face-card-art">${faceMarkup(face, true)}</div><div class="face-card-copy"><p>${face.type}</p><h3>${face.name}</h3><span>${state.face === face.id ? "In use" : "Included"}</span></div><button class="face-select" data-face="${face.id}">${state.face === face.id ? "Current face" : "Use this face"}</button></article>`).join("")}</div></section>`;
}

function settingsMarkup() {
  return `<section class="settings-view settings-view--app"><div class="setting-group"><p class="eyebrow">APP</p>${switchRow("launchToggle", "Launch at login", state.launchAtLogin)}<div class="static-row"><span>Edit objects</span><b>CTRL + SHIFT + E</b></div><div class="static-row"><span>Updates</span><b>0.1.0 · CURRENT</b></div></div><div class="setting-group setting-group--privacy"><p class="eyebrow">ABOUT</p><h3>Presence without obstruction.</h3><p>Timepiece Studio keeps your objects and photos on this device. No accounts, analytics, or cloud uploads.</p></div></section>`;
}

function addObjectMarkup() {
  return `<div class="modal-backdrop" id="closeAdd"><section class="add-panel" role="dialog" aria-modal="true" aria-labelledby="addTitle"><div class="section-bar"><div><p class="eyebrow">NEW DESKTOP OBJECT</p><h2 id="addTitle">Add object</h2></div><button class="icon-close" id="closeAddButton" aria-label="Close">${icon("close")}</button></div><div class="add-grid"><button class="add-choice" data-add-type="clock"><span class="choice-mark">12</span><strong>Clock</strong><small>Show time with a chosen face.</small></button><button class="add-choice" data-add-type="photo"><span class="choice-mark">▧</span><strong>Photo</strong><small>Choose a local PNG, JPEG, or WebP.</small></button><div class="add-choice is-disabled"><span class="choice-mark">Aa</span><strong>Note</strong><small>Coming later</small></div><div class="add-choice is-disabled"><span class="choice-mark">00</span><strong>Timer</strong><small>Coming later</small></div></div></section></div>`;
}

function render() {
  document.querySelector("#app").innerHTML = appMarkup();
  const hover = document.querySelector("#objectHover");
  if (hover) hover.value = state.inspector === "photo" ? state.photoObject.visibilityBehaviour : state.visibilityBehaviour;
  bindEvents();
  syncClock();
}

function setFace(id) { state.face = id; state.selected = id; persist(); render(); }
function toggle(key) { state[key] = !state[key]; persist(); render(); }

function bindEvents() {
  document.querySelectorAll("[data-screen]").forEach((button) => button.addEventListener("click", () => { state.screen = button.dataset.screen; state.inspector = null; render(); }));
  document.querySelectorAll("[data-face]").forEach((element) => element.addEventListener("click", (event) => { const id = event.currentTarget.dataset.face; if (faces.some((face) => face.id === id)) setFace(id); }));
  ["addObject", "addObjectPrimary", "addObjectSecondary", "emptyAdd"].forEach((id) => document.querySelector(`#${id}`)?.addEventListener("click", () => { state.addOpen = true; render(); }));
  document.querySelector("#closeAdd")?.addEventListener("click", (event) => { if (event.target.id === "closeAdd") { state.addOpen = false; render(); } });
  document.querySelector("#closeAddButton")?.addEventListener("click", () => { state.addOpen = false; render(); });
  document.querySelectorAll("[data-add-type]").forEach((button) => button.addEventListener("click", () => addObject(button.dataset.addType)));
  document.querySelectorAll("[data-edit-object]").forEach((button) => button.addEventListener("click", () => { state.screen = "objects"; state.inspector = button.dataset.editObject; render(); }));
  document.querySelectorAll("[data-toggle-object]").forEach((button) => button.addEventListener("click", () => toggleObjectVisibility(button.dataset.toggleObject)));
  document.querySelectorAll("[data-remove-object]").forEach((button) => button.addEventListener("click", () => removeObject(button.dataset.removeObject)));
  document.querySelector("#closeInspector")?.addEventListener("click", () => { state.inspector = null; render(); });
  document.querySelector("#removeInspected")?.addEventListener("click", () => removeObject(state.inspector));
  document.querySelector("#changePhoto")?.addEventListener("click", openPhotoPicker);
  document.querySelector("#objectHover")?.addEventListener("change", (event) => updateInspected({ visibilityBehaviour: event.target.value }));
  document.querySelector("#objectClicks")?.addEventListener("click", () => updateInspected({ clickThrough: !inspectedObject().clickThrough }));
  document.querySelector("#objectTop")?.addEventListener("click", () => updateInspected({ alwaysOnTop: !inspectedObject().alwaysOnTop }));
  document.querySelector("#objectLock")?.addEventListener("click", () => updateInspected({ locked: !inspectedObject().locked }));
  document.querySelector("#objectVisible")?.addEventListener("click", () => updateInspected({ visible: !inspectedObject().visible }));
  document.querySelector("#objectSize")?.addEventListener("input", (event) => updateInspected({ width: Number(event.target.value), ...(state.inspector === "clock" ? { size: Number(event.target.value), height: Number(event.target.value) } : {}) }, false));
  document.querySelector("#objectSize")?.addEventListener("change", () => render());
  document.querySelector("#secondsToggle")?.addEventListener("click", () => updateClock({ showSeconds: !state.showSeconds }));
  document.querySelector("#smoothToggle")?.addEventListener("click", () => updateClock({ smooth: !state.smooth }));
  document.querySelector("#launchToggle")?.addEventListener("click", async () => {
    const enabled = !state.launchAtLogin;
    try {
      if (nativeRuntime) await invoke("set_launch_at_login", { enabled });
      state.launchAtLogin = enabled;
      persist();
      render();
    } catch (error) { console.error("Autostart unavailable", error); }
  });
  document.querySelector("#closeWidget")?.addEventListener("click", () => updateClock({ visible: false }));
}

function inspectedObject() { return state.inspector === "photo" ? state.photoObject : state; }

async function updateClock(changes, redraw = true) {
  Object.assign(state, changes);
  persist();
  if (redraw) render();
}

async function updateInspected(changes, redraw = true) {
  if (state.inspector === "photo") await updatePhotoObject(changes, redraw);
  else await updateClock(changes, redraw);
}

function addObject(type) {
  state.addOpen = false;
  if (type === "photo") return openPhotoPicker();
  state.screen = "objects";
  state.inspector = "clock";
  updateClock({ enabled: true, visible: true });
}

function toggleObjectVisibility(type) {
  if (type === "photo") updatePhotoObject({ visible: !state.photoObject.visible });
  else updateClock({ visible: !state.visible });
}

function removeObject(type) {
  if (type === "photo") updatePhotoObject({ enabled: false, visible: false });
  else updateClock({ enabled: false, visible: false });
  state.inspector = null;
}

async function updatePhotoObject(changes, redraw = true) {
  if (!state.photoObject) return;
  try {
    state.photoObject = await invoke("update_photo_settings", { settings: { ...state.photoObject, ...changes } });
    if (redraw) render();
  } catch (error) { console.error("Could not update photo object", error); }
}

function openPhotoPicker() {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = "image/png,image/jpeg,image/webp";
  input.hidden = true;
  document.body.append(input);
  input.addEventListener("change", () => {
    const file = input.files?.[0];
    if (!file) { input.remove(); return; }
    const reader = new FileReader();
    reader.addEventListener("load", () => {
      const image = new Image();
      image.addEventListener("load", async () => {
        if (nativeRuntime) {
          try {
            state.photoObject = await invoke("import_photo", { photo: { dataUrl: reader.result, naturalWidth: image.naturalWidth, naturalHeight: image.naturalHeight } });
            state.photoPreview = reader.result;
            state.screen = "objects";
            state.inspector = "photo";
            render();
          } catch (error) { console.error("Could not import photo", error); }
          return;
        }
        const scale = Math.min(1, 1000 / Math.max(image.width, image.height));
        const canvas = document.createElement("canvas");
        canvas.width = Math.round(image.width * scale);
        canvas.height = Math.round(image.height * scale);
        canvas.getContext("2d").drawImage(image, 0, 0, canvas.width, canvas.height);
        state.photoPreview = canvas.toDataURL("image/jpeg", .86);
        state.photoObject = { ...defaults, id: "photo", objectType: "photo", width: 420, height: 315, locked: false, enabled: true, visible: true };
        state.screen = "objects";
        state.inspector = "photo";
        persist();
        render();
      });
      image.src = reader.result;
    });
    reader.readAsDataURL(file);
    input.remove();
  });
  input.click();
}

function syncClock() {
  const angles = clockAngles(new Date());
  document.querySelectorAll(".hand--hour").forEach((hand) => hand.style.transform = `rotate(${angles.hour}deg)`);
  document.querySelectorAll(".hand--minute").forEach((hand) => hand.style.transform = `rotate(${angles.minute}deg)`);
  document.querySelectorAll(".hand--second").forEach((hand) => hand.style.transform = `rotate(${angles.second}deg)`);
  document.body.classList.toggle("motion-off", !state.animate);
  document.body.dataset.mode = state.mode;
}

function clockLoop() {
  syncClock();
  if (state.smooth) requestAnimationFrame(clockLoop);
  else setTimeout(clockLoop, 1000);
}

const assetByFace = {
  koi: "/assets/koi-nocturne-alpha.png",
  orbit: "/assets/aurora-orrery.png",
  flower: "/assets/verdant-halo.png",
  amber: "/assets/tangerine-tide.png"
};

function applyRuntimeSettings(settings) {
  Object.assign(state, {
    face: settings.selectedFace,
    x: settings.x,
    y: settings.y,
    size: settings.width,
    monitor: settings.monitor,
    scaleFactor: settings.scaleFactor,
    alwaysOnTop: settings.alwaysOnTop,
    locked: settings.locked,
    visibilityBehaviour: settings.visibilityBehaviour,
    clickThrough: settings.clickThrough,
    ghostHideDelay: settings.ghostHideDelay,
    ghostReturnDelay: settings.ghostReturnDelay,
    fadeOpacity: settings.fadeOpacity,
    enabled: settings.enabled,
    showSeconds: settings.showSecondHand,
    smooth: settings.smoothMovement,
    visible: settings.visible,
    launchAtLogin: settings.launchAtLogin
  });
}

function clockRuntimeMarkup(settings) {
  const face = faces.find((item) => item.id === settings.selectedFace) || faces[0];
  const image = assetByFace[face.id] || assetByFace.koi;
  return `<main class="clock-object" id="clockObject" aria-label="${face.name} floating clock">
    <img class="clock-face-image" src="${image}" alt="" draggable="false" />
    <div class="runtime-hands" aria-hidden="true">
      <i class="hand hand--hour"></i><i class="hand hand--minute"></i>
      <i class="hand hand--second ${settings.showSecondHand ? "" : "is-hidden"}"></i><span class="hub"></span>
    </div>
    <div class="edit-controls" aria-label="Clock edit controls">
      <button class="object-control object-move" id="objectMove" aria-label="Move clock" title="Move clock">${icon("move", 15)}</button>
      <button class="object-control object-settings" id="objectSettings" aria-label="Finish editing" title="Finish editing">${icon("settings", 15)}</button>
      <button class="object-control object-close" id="objectClose" aria-label="Hide clock" title="Hide clock">${icon("close", 15)}</button>
      <button class="object-control object-resize" id="objectResize" aria-label="Resize clock" title="Resize clock">${icon("expand", 15)}</button>
    </div>
    <output class="debug-readout" id="debugReadout" aria-live="off"></output>
  </main>`;
}

function bindClockControls(settings) {
  document.querySelector("#objectMove")?.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    invoke("start_clock_drag").catch(console.error);
  });
  document.querySelector("#objectSettings")?.addEventListener("click", () => invoke("open_object_inspector", { label: "clock" }).catch(console.error));
  document.querySelector("#objectClose")?.addEventListener("click", () => {
    settings.visible = false;
    invoke("update_settings", { settings }).catch(console.error);
  });
  const handle = document.querySelector("#objectResize");
  handle?.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    invoke("start_clock_resize").catch(console.error);
  });
}

function photoRuntimeMarkup(data) {
  return `<main class="clock-object photo-object" id="photoObject" aria-label="Floating photo">
    <img class="photo-object-image" src="${data}" alt="Your floating local photo" draggable="false" />
    <div class="edit-controls" aria-label="Photo edit controls">
      <button class="object-control object-move" id="objectMove" aria-label="Move photo" title="Move photo">${icon("move", 15)}</button>
      <button class="object-control object-settings" id="objectSettings" aria-label="Finish editing" title="Finish editing">${icon("settings", 15)}</button>
      <button class="object-control object-close" id="objectClose" aria-label="Hide photo" title="Hide photo">${icon("close", 15)}</button>
      <button class="object-control object-resize" id="objectResize" aria-label="Resize photo" title="Resize photo">${icon("expand", 15)}</button>
    </div>
    <output class="debug-readout" id="debugReadout" aria-live="off"></output>
  </main>`;
}

function bindPhotoControls(settings) {
  document.querySelector("#objectMove")?.addEventListener("pointerdown", (event) => { event.preventDefault(); invoke("start_object_drag", { label: "photo" }).catch(console.error); });
  document.querySelector("#objectSettings")?.addEventListener("click", () => invoke("open_object_inspector", { label: "photo" }).catch(console.error));
  document.querySelector("#objectClose")?.addEventListener("click", () => invoke("update_photo_settings", { settings: { ...settings, visible: false } }).catch(console.error));
  document.querySelector("#objectResize")?.addEventListener("pointerdown", (event) => { event.preventDefault(); invoke("start_object_resize", { label: "photo" }).catch(console.error); });
}

function bindObjectAppearance(id) {
  return listen("object-appearance", ({ payload }) => {
    const object = document.querySelector(`#${id}`);
    if (!object) return;
    object.style.transitionDuration = `${payload.durationMs}ms`;
    object.style.opacity = String(payload.opacity);
    object.dataset.interactionState = payload.state;
  });
}

async function bootClockRuntime() {
  document.body.className = "clock-runtime";
  let settings = await invoke("get_settings");
  const draw = () => {
    document.querySelector("#app").innerHTML = clockRuntimeMarkup(settings);
    document.body.classList.toggle("smooth-seconds", settings.smoothMovement);
    bindClockControls(settings);
    syncClock();
  };
  draw();
  await listen("runtime-settings", ({ payload }) => { settings = payload; draw(); });
  await listen("edit-mode", ({ payload }) => document.body.classList.toggle("is-editing", payload));
  await bindObjectAppearance("clockObject");
  if (import.meta.env.DEV) await listen("debug-snapshot", ({ payload }) => {
    const output = document.querySelector("#debugReadout");
    if (output) output.textContent = `${payload.state} · cursor ${Math.round(payload.cursorX)},${Math.round(payload.cursorY)} · window ${payload.windowX},${payload.windowY} ${payload.width}×${payload.height} · inside ${payload.cursorInsideBounds} · click-through ${payload.ignoreCursorEvents} · ${payload.monitor || "monitor"} @${payload.scaleFactor}`;
  });
  clockLoop();
}

async function bootPhotoRuntime() {
  document.body.className = "clock-runtime photo-runtime";
  let settings = await invoke("get_photo_settings");
  let data = await invoke("get_photo_data");
  if (!settings || !data) return;
  const draw = () => {
    document.querySelector("#app").innerHTML = photoRuntimeMarkup(data);
    bindPhotoControls(settings);
  };
  draw();
  await listen("photo-settings", ({ payload }) => { settings = payload; draw(); });
  await listen("photo-data", ({ payload }) => { data = payload; draw(); });
  await listen("edit-mode", ({ payload }) => document.body.classList.toggle("is-editing", payload));
  await bindObjectAppearance("photoObject");
}

async function bootStudioRuntime() {
  // Subscribe before taking the initial snapshot. A Photo can be imported while
  // the Studio is loading; the final snapshot keeps the Studio descriptive and
  // never makes its temporary UI state responsible for the native object.
  await Promise.all([
    listen("runtime-settings", ({ payload }) => { applyRuntimeSettings(payload); render(); }),
    listen("photo-settings", ({ payload }) => { state.photoObject = payload; render(); }),
    listen("photo-data", ({ payload }) => { state.photoPreview = payload; render(); }),
  ]);
  try {
    const [settings, photoObject, photoPreview] = await Promise.all([
      invoke("get_settings"),
      invoke("get_photo_settings"),
      invoke("get_photo_data"),
    ]);
    applyRuntimeSettings(settings);
    state.photoObject = photoObject;
    state.photoPreview = photoPreview;
  } catch (error) { console.error(error); }
  render();
  await listen("open-object-inspector", ({ payload }) => { state.screen = "objects"; state.inspector = payload; render(); });
}

if (clockWindow && nativeRuntime) bootClockRuntime().catch(console.error);
else if (photoWindow && nativeRuntime) bootPhotoRuntime().catch(console.error);
else {
  clockLoop();
  if (nativeRuntime) bootStudioRuntime().catch(console.error);
  else render();
}
