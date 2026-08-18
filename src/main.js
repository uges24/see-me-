import "./style.css";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { clockAngles } from "./clock.js";

const nativeRuntime = isTauri();
const clockWindow = new URLSearchParams(location.search).get("window") === "clock";

const faces = [
  { id: "koi", name: "Koi Nocturne", type: "Koi", tone: "indigo", free: true, note: "Original ornamental dial" },
  { id: "orbit", name: "Aurora Orrery", type: "Orbit", tone: "teal", free: true, note: "Layered celestial metalwork" },
  { id: "flower", name: "Verdant Halo", type: "Flower", tone: "lime", free: true, note: "Translucent cast resin" },
  { id: "amber", name: "Tangerine Tide", type: "Fish", tone: "amber", free: true, note: "Marbled artisan resin" },
  { id: "asap", name: "Daydream Coast", type: "Clay", tone: "mint", free: true, note: "Hand-sculpted miniature world" },
  { id: "love", name: "Love Frame", type: "Love", tone: "rose", free: true, note: "Your photo, on device" }
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
  x: 80,
  y: 80,
  monitor: null,
  scaleFactor: 1,
  locked: true,
  behaviour: "ghost",
  ghostHideDelay: 0,
  ghostReturnDelay: 150,
  fadeOpacity: .15,
  visible: true,
  launchAtLogin: false
};

let saved = {};
try { saved = JSON.parse(localStorage.getItem("timepiece-studio") || "{}"); } catch { localStorage.removeItem("timepiece-studio"); }
const state = { ...defaults, ...saved, screen: "studio", selected: "koi", widgetOpen: true };

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
  behaviour: state.behaviour,
  ghostHideDelay: state.ghostHideDelay,
  ghostReturnDelay: state.ghostReturnDelay,
  fadeOpacity: state.fadeOpacity,
  showSecondHand: state.showSeconds,
  smoothMovement: state.smooth,
  visible: state.visible,
  launchAtLogin: state.launchAtLogin
});

const persist = () => {
  const { screen, selected, widgetOpen, ...stored } = state;
  localStorage.setItem("timepiece-studio", JSON.stringify(stored));
  if (nativeRuntime && !clockWindow) invoke("update_settings", { settings: runtimeSettings() }).catch(console.error);
};

function faceMarkup(face, compact = false) {
  const photo = state.photo ? `<div class="photo-fill" style="background-image:url('${state.photo}');background-size:${state.photoScale * 100}%;background-position:${state.photoX}% ${state.photoY}%"></div>` : '<div class="photo-placeholder">YOUR MEMORY</div>';
  const sizeClass = compact ? "dial--compact" : "";
  const art = {
    Koi: `<div class="ornate-ring"></div><div class="wave wave--one"></div><div class="wave wave--two"></div><div class="koi koi--one"></div><div class="koi koi--two"></div><div class="diamond diamond--one"></div><div class="diamond diamond--two"></div>`,
    Orbit: `<div class="orbit-core"></div><div class="orbit orbit--one"></div><div class="orbit orbit--two"></div><i class="planet planet--one"></i><i class="planet planet--two"></i><i class="planet planet--three"></i>`,
    Flower: `<div class="petal petal--a"></div><div class="petal petal--b"></div><div class="petal petal--c"></div><div class="petal petal--d"></div><div class="petal petal--e"></div><div class="flower-core"></div>`,
    Amber: `<div class="amber-vein vein--one"></div><div class="amber-vein vein--two"></div><div class="amber-vein vein--three"></div><div class="amber-speck speck--one"></div><div class="amber-speck speck--two"></div>`,
    ASAP: `<div class="asap-copy">WE NEED THIS<br><b>ASAP</b></div><div class="sleepy-bed"></div><div class="urgent-stamp">URGENT</div>`,
    Love: `<div class="heart-shell"></div><div class="heart-photo">${photo}</div><div class="heart-glint"></div>`
  }[face.type];
  return `<div class="dial dial--${face.id} ${sizeClass}" data-face="${face.id}" aria-label="${face.name} live analogue clock"><div class="dial-art">${art}</div><div class="numerals"><b>12</b><b>3</b><b>6</b><b>9</b></div><div class="ticks"></div><div class="hands"><i class="hand hand--hour"></i><i class="hand hand--minute"></i><i class="hand hand--second ${state.showSeconds ? "" : "is-hidden"}"></i><span class="hub"></span></div></div>`;
}

function appMarkup() {
  const active = faces.find((face) => face.id === state.face);
  return `<main class="shell">
    <aside class="rail" aria-label="Primary navigation">
      <div class="brand">T<span>/</span></div>
      <nav>
        <button class="rail-button ${state.screen === "studio" ? "is-active" : ""}" data-screen="studio" aria-label="Studio">${icon("grid")}</button>
        <button class="rail-button ${state.screen === "gallery" ? "is-active" : ""}" data-screen="gallery" aria-label="Face gallery">${icon("gallery")}</button>
        <button class="rail-button ${state.screen === "settings" ? "is-active" : ""}" data-screen="settings" aria-label="Settings">${icon("settings")}</button>
      </nav>
      <button class="rail-button rail-button--bottom" id="toggleWidget" aria-label="Toggle floating clock">${icon("spark")}</button>
    </aside>
    <section class="workspace">
      <header class="topbar"><div><p class="eyebrow">TIMEPIECE STUDIO</p><h1>${state.screen === "studio" ? "A clock with presence." : state.screen === "gallery" ? "Choose the atmosphere." : "Set the rhythm."}</h1></div><div class="topbar-meta"><span class="live-dot"></span><span>LOCAL ONLY</span></div></header>
      ${state.screen === "studio" ? studioMarkup(active) : state.screen === "gallery" ? galleryMarkup() : settingsMarkup()}
    </section>
    ${!nativeRuntime && state.widgetOpen ? widgetMarkup(active) : ""}
  </main>`;
}

function studioMarkup(active) {
  return `<section class="studio-view">
    <div class="stage"><div class="stage-light"></div><div class="stage-copy"><p class="eyebrow">CURRENT FACE</p><h2>${active.name}</h2><p>${active.note}. Every face is included, stored locally, and ready to use.</p><button class="button button--light" data-screen="gallery">Browse all faces</button></div><div class="stage-dial">${faceMarkup(active)}</div></div>
    <div class="control-strip"><button class="quick-control" id="secondsToggle"><span>SECOND HAND</span><strong>${state.showSeconds ? "ON" : "OFF"}</strong></button><button class="quick-control" id="animationToggle"><span>FACE MOTION</span><strong>${state.animate ? "ON" : "OFF"}</strong></button><button class="quick-control" data-screen="settings"><span>WINDOW OPACITY</span><strong>${Math.round(state.opacity * 100)}%</strong></button></div>
  </section>`;
}

function galleryMarkup() {
  return `<section class="gallery-view"><div class="gallery-intro"><p>Six original faces. No lock states, no checkout maze, no fake free tier.</p><button class="button button--outline" id="openPhoto">Create a photo face</button></div><div class="face-grid">${faces.map((face, index) => `<article class="face-card ${state.face === face.id ? "is-selected" : ""}" style="--delay:${index * 65}ms"><div class="face-card-art">${faceMarkup(face, true)}</div><div class="face-card-copy"><p>${face.type}</p><h3>${face.name}</h3><span>Included</span></div><button class="face-select" data-face="${face.id}">Use this face</button></article>`).join("")}</div></section>`;
}

function settingsMarkup() {
  const control = (id, label, value) => `<button class="setting-row" id="${id}"><span>${label}</span><b>${value}</b></button>`;
  const delayOptions = [0, 100, 250, 500].map((value) => `<option value="${value}">${value} ms</option>`).join("");
  const photoControls = state.face === "love" ? `<div class="setting-group"><p class="eyebrow">PHOTO FRAME</p><label class="range-row"><span>Photo scale <b>${Math.round(state.photoScale * 100)}%</b></span><input id="photoScale" type="range" min="70" max="170" value="${Math.round(state.photoScale * 100)}" /></label><label class="range-row"><span>Horizontal crop <b>${state.photoX}%</b></span><input id="photoX" type="range" min="0" max="100" value="${state.photoX}" /></label><label class="range-row"><span>Vertical crop <b>${state.photoY}%</b></span><input id="photoY" type="range" min="0" max="100" value="${state.photoY}" /></label></div>` : "";
  return `<section class="settings-view"><div class="settings-column"><div class="setting-group"><p class="eyebrow">INTERACTION</p><label class="select-row"><span>Behaviour</span><select id="behaviourSelect"><option value="ghost">Ghost on hover</option><option value="fade">Fade on hover</option><option value="click-through">Click through</option><option value="stay">Stay visible</option></select></label><label class="select-row"><span>Hide delay</span><select id="hideDelay">${delayOptions}</select></label><label class="select-row"><span>Return delay</span><select id="returnDelay">${delayOptions}</select></label><label class="range-row"><span>Fade opacity <b>${Math.round(state.fadeOpacity * 100)}%</b></span><input id="fadeOpacity" type="range" min="5" max="50" value="${Math.round(state.fadeOpacity * 100)}" /></label>${nativeRuntime ? control("editToggle", "Edit clock", "CTRL + SHIFT + E") : "<p class='native-note'>Desktop overlay controls are available in the Timepiece Studio Windows app.</p>"}</div><div class="setting-group"><p class="eyebrow">MOTION</p>${control("secondsToggle", "Show second hand", state.showSeconds ? "ON" : "OFF")}${control("smoothToggle", "Smooth movement", state.smooth ? "ON" : "OFF")}${control("animationToggle", "Animate watch face", state.animate ? "ON" : "OFF")}</div><div class="setting-group"><p class="eyebrow">RENDERING</p><label class="select-row"><span>Dial finish</span><select id="renderMode"><option value="smooth">Smooth mineral</option><option value="crisp">Crisp edges</option><option value="retro">Retro grain</option></select></label></div></div><div class="settings-column"><div class="setting-group"><p class="eyebrow">WINDOW</p>${control("visibilityToggle", state.visible ? "Hide clock" : "Show clock", state.visible ? "VISIBLE" : "HIDDEN")}${control("topToggle", "Always above windows", state.alwaysOnTop ? "ON" : "OFF")}${control("lockToggle", "Lock position", state.locked ? "ON" : "OFF")}${control("launchToggle", "Launch at login", state.launchAtLogin ? "ON" : "OFF")}<label class="range-row"><span>Studio preview opacity <b>${Math.round(state.opacity * 100)}%</b></span><input id="opacityRange" type="range" min="45" max="100" value="${Math.round(state.opacity * 100)}" /></label><label class="range-row"><span>Clock size <b>${state.size}px</b></span><input id="sizeRange" type="range" min="180" max="720" value="${state.size}" /></label></div>${photoControls}<div class="setting-group setting-group--privacy"><p class="eyebrow">YOUR DATA</p><h3>Nothing leaves this device.</h3><p>Faces, position, behavior, and your optional photo stay in local application storage. There are no accounts, analytics, or cloud uploads.</p></div></div></section>`;
}

function widgetMarkup(face) {
  return `<aside class="floating-widget ${state.alwaysOnTop ? "is-pinned" : ""}" style="--widget-size:${state.size}px;opacity:${state.opacity}" aria-label="Floating clock widget"><div class="widget-drag" title="Drag in a packaged desktop build">${icon("move", 15)}</div><button class="widget-settings" data-screen="settings" aria-label="Open clock settings">${icon("settings", 16)}</button><button class="widget-close" id="closeWidget" aria-label="Close floating clock">${icon("close", 16)}</button>${faceMarkup(face)}<span class="widget-caption">${face.name}</span></aside>`;
}

function render() {
  document.querySelector("#app").innerHTML = appMarkup();
  const mode = document.querySelector("#renderMode");
  if (mode) mode.value = state.mode;
  const behaviour = document.querySelector("#behaviourSelect");
  if (behaviour) behaviour.value = state.behaviour;
  const hideDelay = document.querySelector("#hideDelay");
  if (hideDelay) hideDelay.value = String(state.ghostHideDelay);
  const returnDelay = document.querySelector("#returnDelay");
  if (returnDelay) returnDelay.value = String(state.ghostReturnDelay);
  bindEvents();
  syncClock();
}

function setFace(id) { state.face = id; state.selected = id; persist(); render(); }
function toggle(key) { state[key] = !state[key]; persist(); render(); }

function bindEvents() {
  document.querySelectorAll("[data-screen]").forEach((button) => button.addEventListener("click", () => { state.screen = button.dataset.screen; render(); }));
  document.querySelectorAll("[data-face]").forEach((element) => element.addEventListener("click", (event) => { const id = event.currentTarget.dataset.face; if (faces.some((face) => face.id === id)) setFace(id); }));
  document.querySelector("#secondsToggle")?.addEventListener("click", () => toggle("showSeconds"));
  document.querySelector("#smoothToggle")?.addEventListener("click", () => toggle("smooth"));
  document.querySelector("#animationToggle")?.addEventListener("click", () => toggle("animate"));
  document.querySelector("#topToggle")?.addEventListener("click", () => toggle("alwaysOnTop"));
  document.querySelector("#lockToggle")?.addEventListener("click", () => toggle("locked"));
  document.querySelector("#visibilityToggle")?.addEventListener("click", () => toggle("visible"));
  document.querySelector("#editToggle")?.addEventListener("click", () => invoke("toggle_edit").catch(console.error));
  document.querySelector("#launchToggle")?.addEventListener("click", async () => {
    const enabled = !state.launchAtLogin;
    try {
      if (nativeRuntime) await invoke("set_launch_at_login", { enabled });
      state.launchAtLogin = enabled;
      persist();
      render();
    } catch (error) { console.error("Autostart unavailable", error); }
  });
  document.querySelector("#toggleWidget")?.addEventListener("click", () => { state.widgetOpen = !state.widgetOpen; render(); });
  document.querySelector("#closeWidget")?.addEventListener("click", () => { state.widgetOpen = false; render(); });
  document.querySelector("#renderMode")?.addEventListener("change", (event) => { state.mode = event.target.value; persist(); document.body.dataset.mode = state.mode; });
  document.querySelector("#behaviourSelect")?.addEventListener("change", (event) => { state.behaviour = event.target.value; persist(); render(); });
  document.querySelector("#hideDelay")?.addEventListener("change", (event) => { state.ghostHideDelay = Number(event.target.value); persist(); });
  document.querySelector("#returnDelay")?.addEventListener("change", (event) => { state.ghostReturnDelay = Number(event.target.value); persist(); });
  document.querySelector("#fadeOpacity")?.addEventListener("input", (event) => { state.fadeOpacity = Number(event.target.value) / 100; persist(); render(); });
  document.querySelector("#opacityRange")?.addEventListener("input", (event) => { state.opacity = event.target.value / 100; persist(); render(); });
  document.querySelector("#sizeRange")?.addEventListener("input", (event) => { state.size = Number(event.target.value); persist(); render(); });
  ["photoScale", "photoX", "photoY"].forEach((id) => document.querySelector(`#${id}`)?.addEventListener("input", (event) => { state[id] = id === "photoScale" ? event.target.value / 100 : Number(event.target.value); persist(); render(); }));
  document.querySelector("#openPhoto")?.addEventListener("click", openPhotoPicker);
}

function openPhotoPicker() {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = "image/*";
  input.addEventListener("change", () => {
    const file = input.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.addEventListener("load", () => {
      const image = new Image();
      image.addEventListener("load", () => {
        const scale = Math.min(1, 1000 / Math.max(image.width, image.height));
        const canvas = document.createElement("canvas");
        canvas.width = Math.round(image.width * scale);
        canvas.height = Math.round(image.height * scale);
        canvas.getContext("2d").drawImage(image, 0, 0, canvas.width, canvas.height);
        state.photo = canvas.toDataURL("image/jpeg", .86);
        state.face = "love";
        persist();
        render();
      });
      image.src = reader.result;
    });
    reader.readAsDataURL(file);
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
  amber: "/assets/tangerine-tide.png",
  asap: "/assets/daydream-coast.png"
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
    behaviour: settings.behaviour,
    ghostHideDelay: settings.ghostHideDelay,
    ghostReturnDelay: settings.ghostReturnDelay,
    fadeOpacity: settings.fadeOpacity,
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
  document.querySelector("#objectSettings")?.addEventListener("click", () => invoke("toggle_edit").catch(console.error));
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
  await listen("clock-appearance", ({ payload }) => {
    const object = document.querySelector("#clockObject");
    if (!object) return;
    object.style.transitionDuration = `${payload.durationMs}ms`;
    object.style.opacity = String(payload.opacity);
    object.dataset.interactionState = payload.state;
  });
  if (import.meta.env.DEV) await listen("debug-snapshot", ({ payload }) => {
    const output = document.querySelector("#debugReadout");
    if (output) output.textContent = `${payload.state} · cursor ${Math.round(payload.cursorX)},${Math.round(payload.cursorY)} · window ${payload.windowX},${payload.windowY} ${payload.width}×${payload.height} · inside ${payload.cursorInsideBounds} · click-through ${payload.ignoreCursorEvents} · ${payload.monitor || "monitor"} @${payload.scaleFactor}`;
  });
  clockLoop();
}

async function bootStudioRuntime() {
  try { applyRuntimeSettings(await invoke("get_settings")); } catch (error) { console.error(error); }
  render();
  await listen("runtime-settings", ({ payload }) => { applyRuntimeSettings(payload); render(); });
}

if (clockWindow && nativeRuntime) bootClockRuntime().catch(console.error);
else {
  clockLoop();
  if (nativeRuntime) bootStudioRuntime().catch(console.error);
  else render();
}
