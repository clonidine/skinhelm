import init, { SkinhelmViewer } from "./pkg/skinhelm_wasm.js";

const status = document.getElementById("status");
const canvas = document.getElementById("viewer-canvas");
const playerInput = document.getElementById("player-input");
const loadPlayer = document.getElementById("load-player");
const skinFile = document.getElementById("skin-file");
const capeFile = document.getElementById("cape-file");
const loadDefault = document.getElementById("load-default");
const toggleAnimation = document.getElementById("toggle-animation");
const toggleOverlays = document.getElementById("toggle-overlays");
const toggleCape = document.getElementById("toggle-cape");
const toggleSlim = document.getElementById("toggle-slim");
const animationSpeed = document.getElementById("animation-speed");
const dockToggle = document.getElementById("dock-toggle");

function setStatus(message) {
  status.textContent = message;
  status.title = message;
}

new MutationObserver(() => {
  status.title = status.textContent || "";
}).observe(status, { childList: true, characterData: true, subtree: true });

function requestedPreset() {
  return "default";
}

function skinPlayerFromPath() {
  const match = window.location.pathname.match(
    /^\/viewer\/([0-9a-fA-F]{32}|[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}|[A-Za-z0-9_]{3,16})\/?$/,
  );
  return match ? match[1] : null;
}

async function loadPlayerSkin(viewer, player) {
  setStatus(`Loading skin for ${player}...`);
  const encodedPlayer = encodeURIComponent(player);
  const response = await fetch(`/viewer/skin/${encodedPlayer}`, {
    headers: { Accept: "image/png" },
    cache: "no-store",
  });
  if (!response.ok) {
    let message = `skin request failed: ${response.status}`;
    try {
      const body = await response.json();
      if (body && body.error) {
        message = body.error;
      }
    } catch (_) {
    }
    throw new Error(message);
  }
  const bytes = new Uint8Array(await response.arrayBuffer());
  const slim = (response.headers.get("x-skinhelm-model") || "").toLowerCase() === "slim";
  const hasCape = (response.headers.get("x-skinhelm-cape") || "").toLowerCase() === "true";
  toggleSlim.checked = slim;
  viewer.load_skin_bytes_with_model(bytes, slim);
  viewer.clear_cape();
  toggleCape.checked = false;
  if (hasCape) {
    await loadPlayerCape(viewer, player);
    toggleCape.checked = true;
    setStatus(`Loaded ${slim ? "slim" : "classic"} skin and cape for ${player}`);
  } else {
    setStatus(`Loaded ${slim ? "slim" : "classic"} skin for ${player}`);
  }
}

async function loadPlayerCape(viewer, player) {
  const response = await fetch(`/viewer/cape/${encodeURIComponent(player)}`, {
    headers: { Accept: "image/png" },
    cache: "no-store",
  });
  if (response.status === 404) {
    viewer.clear_cape();
    toggleCape.checked = false;
    return;
  }
  if (!response.ok) {
    throw new Error(`cape request failed: ${response.status}`);
  }
  const bytes = new Uint8Array(await response.arrayBuffer());
  viewer.load_cape_bytes(bytes);
  toggleCape.checked = true;
}

function syncAnimationControls(viewer) {
  viewer.set_animation_enabled(toggleAnimation.checked);
  animationSpeed.disabled = !toggleAnimation.checked;
}

function defaultDockOpen() {
  return true;
}

function setDockOpen(open) {
  document.documentElement.dataset.dock = open ? "open" : "closed";
  dockToggle.setAttribute("aria-pressed", String(open));
  dockToggle.setAttribute("aria-label", open ? "Hide controls" : "Show controls");
  dockToggle.title = open ? "Hide controls" : "Show controls";
}

async function loadRequestedPlayer(viewer, requestRender) {
  const player = playerInput.value.trim();
  if (!player) {
    return;
  }

  try {
    await loadPlayerSkin(viewer, player);
    requestRender();
  } catch (error) {
    setStatus(String(error));
  }
}

async function main() {
  await init();
  const viewer = SkinhelmViewer.init();
  viewer.resize();
  syncAnimationControls(viewer);
  const preset = requestedPreset();
  if (preset) {
    document.documentElement.dataset.preset = preset;
    setDockOpen(defaultDockOpen());
    const presetStatus = viewer.set_preset(preset);
    setStatus(presetStatus);
  }
  let pointerActive = false;
  let pendingPointerMove = null;
  let frameRequest = null;

  const requestRender = () => {
    if (frameRequest !== null) {
      return;
    }
    frameRequest = requestAnimationFrame(frame);
  };

  const frame = (timestamp) => {
    frameRequest = null;
    try {
      if (pendingPointerMove) {
        viewer.pointer_move(pendingPointerMove.x, pendingPointerMove.y);
        pendingPointerMove = null;
      }
      viewer.render_frame(timestamp);
    } catch (error) {
      setStatus(String(error));
    }
    if (toggleAnimation.checked) {
      requestRender();
    }
  };

  dockToggle.addEventListener("click", () => {
    setDockOpen(document.documentElement.dataset.dock !== "open");
    requestRender();
  });

  const initialPlayer = skinPlayerFromPath();
  if (initialPlayer) {
    playerInput.value = initialPlayer;
    try {
      await loadPlayerSkin(viewer, initialPlayer);
      requestRender();
    } catch (error) {
      setStatus(String(error));
    }
  }

  loadPlayer.addEventListener("click", () => {
    loadRequestedPlayer(viewer, requestRender);
  });

  playerInput.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      loadRequestedPlayer(viewer, requestRender);
    }
  });

  loadDefault.addEventListener("click", () => {
    try {
      viewer.load_default_skin();
      requestRender();
    } catch (error) {
      setStatus(String(error));
    }
  });

  skinFile.addEventListener("change", async () => {
    const file = skinFile.files && skinFile.files[0];
    if (!file) {
      return;
    }

    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      viewer.load_skin_bytes_with_model(bytes, toggleSlim.checked);
      requestRender();
    } catch (error) {
      setStatus(String(error));
    }
  });

  capeFile.addEventListener("change", async () => {
    const file = capeFile.files && capeFile.files[0];
    if (!file) {
      viewer.set_cape_visible(toggleCape.checked);
      requestRender();
      return;
    }

    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      viewer.load_cape_bytes(bytes);
      toggleCape.checked = true;
      requestRender();
    } catch (error) {
      setStatus(String(error));
    }
  });

  toggleAnimation.addEventListener("change", () => {
    syncAnimationControls(viewer);
    requestRender();
  });

  toggleOverlays.addEventListener("change", () => {
    viewer.set_overlays_enabled(toggleOverlays.checked);
    requestRender();
  });

  toggleCape.addEventListener("change", () => {
    const message = viewer.set_cape_visible(toggleCape.checked);
    if (message === "No cape loaded") {
      toggleCape.checked = false;
    }
    requestRender();
  });

  animationSpeed.addEventListener("input", () => {
    viewer.set_animation_speed(Number(animationSpeed.value));
    requestRender();
  });

  canvas.addEventListener("mousedown", (event) => {
    event.preventDefault();
    pointerActive = true;
    viewer.pointer_down(event.clientX, event.clientY);
    requestRender();
  });

  window.addEventListener("mousemove", (event) => {
    if (!pointerActive) {
      return;
    }
    event.preventDefault();
    pendingPointerMove = { x: event.clientX, y: event.clientY };
    requestRender();
  });

  window.addEventListener("mouseup", () => {
    pointerActive = false;
    pendingPointerMove = null;
    viewer.pointer_up();
    requestRender();
  });

  canvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      viewer.wheel(event.deltaY);
      requestRender();
    },
    { passive: false },
  );

  window.addEventListener("resize", () => {
    try {
      viewer.resize();
      requestRender();
    } catch (error) {
      setStatus(String(error));
    }
  });

  requestRender();
}

main().catch((error) => setStatus(String(error)));
