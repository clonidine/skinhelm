import init, { SkinhelmViewer } from "./pkg/skinhelm_wasm.js";

const status = document.getElementById("status");
const canvas = document.getElementById("viewer-canvas");
const playerInput = document.getElementById("player-input");
const loadPlayer = document.getElementById("load-player");
const skinFile = document.getElementById("skin-file");
const capeFile = document.getElementById("cape-file");
const loadDefault = document.getElementById("load-default");
const saveHead = document.getElementById("save-head");
const toggleAnimation = document.getElementById("toggle-animation");
const toggleOverlays = document.getElementById("toggle-overlays");
const toggleCape = document.getElementById("toggle-cape");
const toggleSlim = document.getElementById("toggle-slim");
const animationSpeed = document.getElementById("animation-speed");
const dockToggle = document.getElementById("dock-toggle");
const compactViewport = window.matchMedia("(max-width: 760px), (pointer: coarse)");
const STATUS_HIDE_DELAY_MS = 3200;
let statusHideTimer = null;

function setStatus(message, options = {}) {
  window.clearTimeout(statusHideTimer);
  status.textContent = message;
  status.title = message;
  status.dataset.visible = message ? "true" : "false";

  if (options.temporary) {
    const visibleMessage = message;
    statusHideTimer = window.setTimeout(() => {
      if (status.textContent === visibleMessage) {
        status.dataset.visible = "false";
        status.removeAttribute("title");
      }
    }, STATUS_HIDE_DELAY_MS);
  }
}

new MutationObserver(() => {
  const message = status.textContent || "";
  status.title = message;
  if (message) {
    status.dataset.visible = "true";
  }
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
    setStatus(`Loaded ${slim ? "slim" : "classic"} skin and cape for ${player}`, {
      temporary: true,
    });
  } else {
    setStatus(`Loaded ${slim ? "slim" : "classic"} skin for ${player}`, { temporary: true });
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

function isCompactViewport() {
  return compactViewport.matches;
}

function defaultDockOpen() {
  return !isCompactViewport();
}

function setDockOpen(open) {
  document.documentElement.dataset.dock = open ? "open" : "closed";
  dockToggle.setAttribute("aria-pressed", String(open));
  dockToggle.setAttribute("aria-label", open ? "Hide controls" : "Show controls");
  dockToggle.title = open ? "Hide controls" : "Show controls";
}

function collapseDockAfterPrimaryAction() {
  if (isCompactViewport()) {
    setDockOpen(false);
  }
}

function syncViewportHeight() {
  const height = window.visualViewport ? window.visualViewport.height : window.innerHeight;
  document.documentElement.style.setProperty("--app-height", `${height}px`);
}

function downloadPngDataUrl(dataUrl, filename) {
  const link = document.createElement("a");
  link.href = dataUrl;
  link.download = filename;
  link.rel = "noopener";
  document.body.append(link);
  link.click();
  link.remove();
}

function headPngFilename() {
  const label = playerInput.value.trim() || "head";
  const safeLabel = label.replace(/[^A-Za-z0-9_-]/g, "-").slice(0, 32) || "head";
  return `skinhelm-${safeLabel}-180x180.png`;
}

async function loadRequestedPlayer(viewer, requestRender) {
  const player = playerInput.value.trim();
  if (!player) {
    return;
  }

  try {
    await loadPlayerSkin(viewer, player);
    collapseDockAfterPrimaryAction();
    requestRender();
  } catch (error) {
    setStatus(String(error));
  }
}

async function main() {
  syncViewportHeight();
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
      setStatus("Loaded default skin", { temporary: true });
      requestRender();
    } catch (error) {
      setStatus(String(error));
    }
  });

  saveHead.addEventListener("click", () => {
    try {
      const dataUrl = viewer.export_head_png_data_url();
      downloadPngDataUrl(dataUrl, headPngFilename());
      setStatus("Saved head PNG at 180x180", { temporary: true });
      requestRender();
    } catch (error) {
      setStatus(String(error));
      requestRender();
    }
  });

  skinFile.addEventListener("change", async () => {
    const file = skinFile.files && skinFile.files[0];
    if (!file) {
      return;
    }

    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const message = viewer.load_skin_bytes(bytes);
      const slim = message.toLowerCase().includes("(slim)");
      toggleSlim.checked = slim;
      setStatus(`Loaded ${slim ? "slim" : "classic"} skin from PNG`, { temporary: true });
      collapseDockAfterPrimaryAction();
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
      setStatus("Loaded cape from PNG", { temporary: true });
      collapseDockAfterPrimaryAction();
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

  const startPointer = (event) => {
    pointerActive = true;
    viewer.pointer_down(event.clientX, event.clientY);
    requestRender();
  };

  const movePointer = (event) => {
    if (!pointerActive) {
      return;
    }
    pendingPointerMove = { x: event.clientX, y: event.clientY };
    requestRender();
  };

  const endPointer = () => {
    pointerActive = false;
    pendingPointerMove = null;
    viewer.pointer_up();
    requestRender();
  };

  if (window.PointerEvent) {
    canvas.addEventListener(
      "pointerdown",
      (event) => {
        if (!event.isPrimary) {
          return;
        }
        event.preventDefault();
        canvas.setPointerCapture?.(event.pointerId);
        startPointer(event);
      },
      { passive: false },
    );

    canvas.addEventListener(
      "pointermove",
      (event) => {
        if (!event.isPrimary) {
          return;
        }
        event.preventDefault();
        movePointer(event);
      },
      { passive: false },
    );

    canvas.addEventListener("pointerup", endPointer);
    canvas.addEventListener("pointercancel", endPointer);
    canvas.addEventListener("lostpointercapture", endPointer);
  } else {
    canvas.addEventListener(
      "touchstart",
      (event) => {
        if (event.touches.length !== 1) {
          return;
        }
        event.preventDefault();
        startPointer(event.touches[0]);
      },
      { passive: false },
    );

    canvas.addEventListener(
      "touchmove",
      (event) => {
        if (event.touches.length !== 1) {
          return;
        }
        event.preventDefault();
        movePointer(event.touches[0]);
      },
      { passive: false },
    );

    canvas.addEventListener("touchend", endPointer);
    canvas.addEventListener("touchcancel", endPointer);

    canvas.addEventListener("mousedown", (event) => {
      event.preventDefault();
      startPointer(event);
    });

    window.addEventListener("mousemove", (event) => {
      if (!pointerActive) {
        return;
      }
      event.preventDefault();
      movePointer(event);
    });

    window.addEventListener("mouseup", endPointer);
  }

  canvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      viewer.wheel(event.deltaY);
      requestRender();
    },
    { passive: false },
  );

  const resizeViewer = () => {
    syncViewportHeight();
    try {
      viewer.resize();
      requestRender();
    } catch (error) {
      setStatus(String(error));
    }
  };

  window.addEventListener("resize", resizeViewer);
  window.visualViewport?.addEventListener("resize", resizeViewer);
  window.visualViewport?.addEventListener("scroll", resizeViewer);

  const handleViewportModeChange = () => {
    setDockOpen(defaultDockOpen());
    resizeViewer();
  };

  if (compactViewport.addEventListener) {
    compactViewport.addEventListener("change", handleViewportModeChange);
  } else {
    compactViewport.addListener(handleViewportModeChange);
  }

  requestRender();
}

main().catch((error) => setStatus(String(error)));
