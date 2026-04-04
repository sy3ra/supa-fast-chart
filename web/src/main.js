import init, { ChartEngine } from '../pkg/chart_engine.js';

async function run() {
  // --- Centralized Chart Colors ---
  const COLORS = {
    background: '#111',
    candleUp: '#00ff00',
    candleDown: '#ff0000',
    grid: '#333',
    axisText: '#888',
    crosshair: '#ffffff',
    crosshairLabel: '#ffffff',
    drawing: '#00FFFF',
    anchorFill: '#FFFFFF',
    anchorStroke: '#00FFFF',
  };

  const wasm = await init(); // Wasm 로딩 & return the instance to access memory

  // Canvas Setup with DPI Support
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d'); // Define globally for render loop to access
  const dpr = window.devicePixelRatio || 1;

  // Initialize Rust Engine
  const engine = new ChartEngine(window.innerWidth, window.innerHeight - 48); // Match CSS height


  // --- Tab Management ---
  const tabBar = document.getElementById('tab-bar');
  const newTabBtn = document.getElementById('new-tab-btn');

  function renderTabs() {
    const tabs = engine.get_tabs_info();
    tabBar.innerHTML = '';

    tabs.forEach(tab => {
      const el = document.createElement('div');
      el.className = `tab ${tab.is_active ? 'active' : ''}`;
      el.innerHTML = `
        <span class="tab-label">${tab.symbol} ${tab.interval}</span>
        <span class="tab-close" data-id="${tab.id}">×</span>
      `;

      el.onclick = (e) => {
        if (e.target.classList.contains('tab-close')) {
          e.stopPropagation();
          engine.close_tab(tab.id);
          chartDataCache.delete(tab.id);

          // Check if active tab changed
          const info = engine.get_tabs_info();
          const newActive = info.find(t => t.is_active);

          if (newActive && newActive.id !== activeChartId) {
            activeChartId = newActive.id;
            if (!chartDataCache.has(newActive.id)) {
                loadData(newActive.id, newActive.interval);
            }
          }
          updateIntervalUI();
          renderTabs();
        } else {
          engine.switch_tab(tab.id);
          activeChartId = tab.id;

          if (!chartDataCache.has(tab.id)) {
              loadData(tab.id, tab.interval);
          } else {
              subscribeBinanceWS(tab.symbol, tab.interval, tab.id);
          }

          // Sync Tool Panels
          refreshMAList();
          syncBBUI();
          syncAutoScaleUI(); // New: Sync Auto Scale button
          updateIntervalUI();

          renderTabs();
        }
      };

      tabBar.appendChild(el);
    });
  }

  newTabBtn.onclick = () => {
    try {
      // DEBUG: Hardcoded for now to avoid prompt issues
      const symbol = "ETHUSDT";
      const interval = "1h";


      const newId = engine.create_tab(symbol, interval);

      activeChartId = newId;
      renderTabs();

      loadData(newId, interval);
      syncAutoScaleUI();
      updateIntervalUI();
    } catch (e) {
      console.error("Error creating tab:", e);
      // alert("Error creating tab: " + e.message); // Removed alert to prevent stuck state
    }
  };

  // Initial Tab Render
  renderTabs();

  // Resize needs to account for tab bar
  function resizeCanvas() {
    const width = window.innerWidth;
    const height = window.innerHeight - 48; // Updated to match new CSS

    // Set display size (css pixels)
    canvas.style.width = width + "px";
    canvas.style.height = height + "px";

    // Set actual size in memory
    canvas.width = Math.floor(width * dpr);
    canvas.height = Math.floor(height * dpr);

    // Normalize
    ctx.scale(dpr, dpr);

    // Notify Rust
    engine.resize(width, height);
  }

  document.querySelector('#app').innerHTML = '';
  document.querySelector('#app').appendChild(canvas);

  // Initial Resize
  resizeCanvas();

  // Listen for resize
  window.addEventListener('resize', resizeCanvas);

  // Global data storage for X-axis labels
  const chartDataCache = new Map();
  let loadingSessionId = 0; // Prevent race conditions

  let isTimeframeChanging = false; // Block history loader during replacement

  // Target initial load counts per interval to fill history
  const INITIAL_LOAD_COUNTS = {
    '1m': 10000,
    '15m': 5000,
    '1h': 5000,
    '4h': 3000,
    '1d': 3000,
    '1w': 2000,
  };

  // Global AbortController for data fetching
  let currentAbortController = null;
  let activeChartId = 1; // Default to ID 1 on startup
  const initializedCharts = new Set(); // Track which charts have valid initial data/view

  // Global WebSocket State
  let globalWs = null;

  function subscribeBinanceWS(symbol, interval, chartId) {
    if (globalWs) {
      globalWs.close();
      globalWs = null;
    }

    const wsUrl = `wss://stream.binance.com:9443/ws/${symbol.toLowerCase()}@kline_${interval}`;
    globalWs = new WebSocket(wsUrl);

    globalWs.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      if (msg.e === 'kline') {
         if (chartId !== activeChartId) {
            return;
         }
        const k = msg.k;
        const time = k.t; // Kline start time (milliseconds)
        
        // Debug
        // console.log(`[WS] time: ${time}, lastCandleTime: ${globalFlatData ? globalFlatData[globalFlatData.length-6] : 'none'}`);
        const o = parseFloat(k.o);
        const h = parseFloat(k.h);
        const l = parseFloat(k.l);
        const c = parseFloat(k.c);
        const v = parseFloat(k.v);
        const isClosed = k.x;

        engine.update_live_candle(chartId, time, o, h, l, c, v, isClosed);

        // Update JS global state if it's the newest candle
        let flatData = chartDataCache.get(chartId);
        if (flatData && flatData.length >= 6) {
          const lastCandleTime = flatData[flatData.length - 6];
          
          if (time === lastCandleTime) {
             // Update existing
             flatData[flatData.length - 5] = o;
             flatData[flatData.length - 4] = h;
             flatData[flatData.length - 3] = l;
             flatData[flatData.length - 2] = c;
             flatData[flatData.length - 1] = v;
          } else if (time > lastCandleTime) {
             const newFlat = new Float64Array(flatData.length + 6);
             newFlat.set(flatData);
             newFlat[newFlat.length - 6] = time;
             newFlat[newFlat.length - 5] = o;
             newFlat[newFlat.length - 4] = h;
             newFlat[newFlat.length - 3] = l;
             newFlat[newFlat.length - 2] = c;
             newFlat[newFlat.length - 1] = v;
             chartDataCache.set(chartId, newFlat);
          }
        }

      }
    };

    globalWs.onerror = (error) => {
      console.error('WebSocket Error:', error);
    };
  }

  async function fetchCandles(interval, endTime = null, limit = 1000, signal = null) {
    let url = `https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=${interval}&limit=${limit}`;
    if (endTime) {
      url += `&endTime=${endTime}`;
    }
    const res = await fetch(url, { signal });
    if (!res.ok) throw new Error(res.statusText);
    return await res.json();
  }

  const INTERVAL_MS = {
    '1m': 60 * 1000,
    '15m': 15 * 60 * 1000,
    '1h': 60 * 60 * 1000,
    '4h': 4 * 60 * 60 * 1000,
    '1d': 24 * 60 * 60 * 1000,
    '1w': 7 * 24 * 60 * 60 * 1000,
  };

  // Helper for throttling
  const sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));

  // Fetch Real Data from Binance
  async function loadData(chartId, interval = '1d', anchorTime = null) {
    try {
      // 1. Cancel previous requests ONLY if targeting active chart?
      // For simplicity, we cancel any ongoing "global" load.
      if (currentAbortController) {
        currentAbortController.abort();
      }
      currentAbortController = new AbortController();
      const signal = currentAbortController.signal;

      isLoading = true; // Block infinite scroll while loading

      const sessionId = ++loadingSessionId;

      if (chartId === activeChartId) {
        updateIntervalUI();
        noMoreHistory = false;
      }

      const targetCount = INITIAL_LOAD_COUNTS[interval] || 1000;

      // 2. Fetch Key Batch (Instant Render)
      const keyBatch = await fetchCandles(interval, anchorTime, 1000, signal);

      if (sessionId !== loadingSessionId) return; // Stale

      if (!keyBatch || keyBatch.length === 0) {
        console.error("No data found");
        return;
      }

      // Initial Render
      const keyFlat = flattenCandles(keyBatch);

      engine.replace_candles(chartId, keyFlat);

      chartDataCache.set(chartId, keyFlat);

      // CONDITIONAL RESET: Only reset view if this is the FIRST load for this chart.
      if (!initializedCharts.has(chartId)) {
        engine.reset_view(chartId);
        initializedCharts.add(chartId);
        if (chartId === activeChartId) {
          syncAutoScaleUI();
        }
      }

      if (keyBatch.length < 1000 || targetCount <= 1000) {
        return;
      }

      // 3. Chunked Parallel Fetch for History (Deep Fill)
      const intervalMs = INTERVAL_MS[interval];
      const earliestTime = keyBatch[0][0];
      const remainingCount = targetCount - keyBatch.length;
      const batchesNeeded = Math.ceil(remainingCount / 1000);

      const CONCURRENT_LIMIT = 3;
      let combinedHistory = [];

      // Calculate all needed endTimes first
      const tasks = [];
      for (let i = 0; i < batchesNeeded; i++) {
        const endTime = (earliestTime - 1) - (i * 1000 * intervalMs);
        tasks.push(endTime);
      }

      // Process in chunks
      for (let i = 0; i < tasks.length; i += CONCURRENT_LIMIT) {
        if (sessionId !== loadingSessionId) break;

        const chunk = tasks.slice(i, i + CONCURRENT_LIMIT);

        const chunkPromises = chunk.map(endTime => fetchCandles(interval, endTime, 1000, signal));
        const chunkResults = await Promise.all(chunkPromises);

        // Collect results
        for (const batch of chunkResults) {
          if (batch && batch.length > 0) {
            combinedHistory = combinedHistory.concat(batch);
          }
        }

        // Tiny throttle between chunks to be nice to API
        await sleep(50);
      }

      if (sessionId !== loadingSessionId) return;

      if (combinedHistory.length > 0) {
        // 4. Sort to ensure integrity (API async nature)
        combinedHistory.sort((a, b) => a[0] - b[0]);

        const historyFlat = flattenCandles(combinedHistory);

        // Prepend to Rust
        engine.prepend_candles(chartId, historyFlat);

        // Update Global JS Data
        const existingData = chartDataCache.get(chartId) || new Float64Array(0);
        const totalFlat = new Float64Array(historyFlat.length + existingData.length);
        totalFlat.set(historyFlat);
        totalFlat.set(existingData, historyFlat.length);
        chartDataCache.set(chartId, totalFlat);
      }



    } catch (e) {
      if (e.name === 'AbortError') {
        console.log('Fetch aborted');
      } else {
        console.error("Failed to load data", e);
      }
    } finally {
      // Only clear loading flag if THIS session is still valid
      // (Actually simply clearing is safer, next load sets it true again)
      isLoading = false;

      // 5. Start Real-time WebSocket Stream for the Active Chart
      // Move this to finally so it runs even if we early return due to targetCount <= 1000
      if (chartId === activeChartId) {
        const tabs = engine.get_tabs_info();
        const currentTab = tabs.find(t => t.id === chartId);
        const symbol = currentTab ? currentTab.symbol : "BTCUSDT"; // Default fallback
        subscribeBinanceWS(symbol, interval, chartId);
      }
    }
  }

  async function changeTimeframe(newInterval) {
    isTimeframeChanging = true;

    try {
      let anchorTime = null;
      const viewState = engine.get_view_state(); // [min, max, start, end]
      const endIdx = viewState[3];
      const flatData = chartDataCache.get(activeChartId);

      if (flatData) {
        // Map endIdx to globalFlatData index
        const len = flatData.length / 6;
        const safeIdx = Math.min(Math.floor(endIdx), len - 1);
        if (safeIdx >= 0) {
          anchorTime = flatData[safeIdx * 6];
        }
      }

      engine.set_tab_interval(activeChartId, newInterval);
      chartDataCache.delete(activeChartId);

      // 2. Load Data (anchored)
      await loadData(activeChartId, newInterval, anchorTime);

      // 3. Let the Engine handle View Restoration based on anchored Time
      initializedCharts.add(activeChartId); // Mark as clean

    } catch (e) {
      console.error("Timeframe switch failed", e);
    } finally {
      isTimeframeChanging = false;
    }
  }

  function flattenCandles(data) {
    const flatData = new Float64Array(data.length * 6);
    for (let i = 0; i < data.length; i++) {
      const k = data[i];
      const offset = i * 6;
      flatData[offset] = Number(k[0]);
      flatData[offset + 1] = Number(k[1]);
      flatData[offset + 2] = Number(k[2]);
      flatData[offset + 3] = Number(k[3]);
      flatData[offset + 4] = Number(k[4]);
      flatData[offset + 5] = Number(k[5]);
    }
    return flatData;
  }

  // --- Toolbar: Interval Selector ---
  const toolbar = document.createElement('div');
  Object.assign(toolbar.style, {
    position: 'absolute',
    top: '60px',
    left: '10px',
    zIndex: '100',
    display: 'flex',
    gap: '10px',
  });

  // Interval Group
  const intervalGroup = createToolbarGroup();
  const intervals = ['1m', '15m', '1h', '4h', '1d', '1w'];
  const intervalButtons = {};

  intervals.forEach(int => {
    // Call changeTimeframe instead of loadData
    const btn = createToolbarButton(int, () => changeTimeframe(int));
    btn.style.minWidth = '25px';
    intervalButtons[int] = btn;
    intervalGroup.appendChild(btn);
  });

  function updateIntervalUI() {
    const info = engine.get_tabs_info();
    const activeTab = info.find(t => t.is_active);
    const activeInterval = activeTab ? activeTab.interval : '1d';

    intervals.forEach(int => {
      const btn = intervalButtons[int];
      if (int === activeInterval) {
        btn.style.color = '#00ff00';
        btn.style.fontWeight = 'bold';
        btn.style.borderBottom = '1px solid #00ff00';
      } else {
        btn.style.color = '#ccc';
        btn.style.fontWeight = 'normal';
        btn.style.borderBottom = 'none';
      }
    });
  }

  toolbar.appendChild(intervalGroup);



  // --- UI Toolbar ---
  // (toolbar already created above)

  const tools = [
    { name: 'None', type: -1 },
    { name: 'Segment', type: 0 },
    { name: 'Line', type: 1 },
    { name: 'Ray', type: 2 },
    { name: 'Horz', type: 3 },
    { name: 'Vert', type: 4 },
  ];

  let currentToolType = -1; // -1 = None/Idle
  let isDrawingActive = false;
  const toolButtons = []; // Store buttons to update styles

  // Helper functions (Hoisted)
  function createToolbarGroup() {
    const group = document.createElement('div');
    group.style.display = 'flex';
    group.style.gap = '2px';
    group.style.background = '#222';
    group.style.padding = '4px';
    group.style.borderRadius = '4px';
    group.style.border = '1px solid #444';
    return group;
  }

  function createToolbarButton(text, onClick) {
    const btn = document.createElement('button');
    btn.innerText = text;
    btn.style.padding = '5px 8px';
    btn.style.border = '1px solid transparent';
    btn.style.background = 'transparent';
    btn.style.color = '#ccc';
    btn.style.cursor = 'pointer';
    btn.style.borderRadius = '3px';
    btn.style.fontSize = '12px';
    btn.onclick = onClick;

    btn.onmouseover = () => btn.style.background = '#333';
    btn.onmouseout = () => btn.style.background = 'transparent';

    return btn;
  }

  // --- Drawing Tools Group ---
  const drawingGroup = createToolbarGroup();
  toolbar.appendChild(drawingGroup);

  tools.forEach(t => {
    const btn = createToolbarButton(t.name, () => {
      engine.deselect_drawing(); // Deselect any active drawing
      currentToolType = t.type;
      isDrawingActive = false; // Reset potential active state

      // Update UI
      toolButtons.forEach(b => {
        b.style.background = 'transparent';
        b.style.borderColor = 'transparent';
        b.style.color = '#ccc';
      });
      btn.style.background = '#007bff'; // Active Blue
      btn.style.color = '#fff';
    });

    // Remove hover effects for tool buttons as they handle their own active state
    btn.onmouseover = null;
    btn.onmouseout = null;

    if (t.type === -1) {
      btn.style.fontWeight = 'bold';
    }

    drawingGroup.appendChild(btn);
    toolButtons.push(btn);
  });

  toolbar.appendChild(drawingGroup);

  // Set initial active state (None)
  if (toolButtons.length > 0) {
    toolButtons[0].click();
  }

  // --- History Group (Undo/Redo) ---
  const historyGroup = createToolbarGroup();

  const undoBtn = createToolbarButton('< Undo', () => engine.undo());
  historyGroup.appendChild(undoBtn);

  const redoBtn = createToolbarButton('Redo >', () => engine.redo());
  historyGroup.appendChild(redoBtn);

  toolbar.appendChild(historyGroup);

  // --- Settings Group (Scale) ---
  const settingsGroup = createToolbarGroup();

  const autoScaleBtn = createToolbarButton('Scale: Auto', () => {
    const isAuto = engine.get_auto_scale();
    engine.set_auto_scale(!isAuto);
    updateAutoScaleUI();
  });

  // Custom style for toggle
  autoScaleBtn.style.minWidth = "80px";

  // Override hover for toggle
  autoScaleBtn.onmouseover = null;
  autoScaleBtn.onmouseout = null;

  function updateAutoScaleUI() {
    const isAuto = engine.get_auto_scale();
    autoScaleBtn.innerText = isAuto ? 'Scale: Auto' : 'Scale: Manual';
    // When active, use blue. When manual, transparent/gray
    autoScaleBtn.style.background = isAuto ? '#007bff' : 'transparent';
    autoScaleBtn.style.color = isAuto ? '#fff' : '#ccc';
  }

  // Sync from Rust (for Tab Switching)
  function syncAutoScaleUI() {
    const status = engine.get_active_chart_status();
    if (status) { // { is_auto_scale: bool, ... }
      const isAuto = status.is_auto_scale;
      autoScaleBtn.innerText = isAuto ? 'Scale: Auto' : 'Scale: Manual';
      autoScaleBtn.style.background = isAuto ? '#007bff' : 'transparent';
      autoScaleBtn.style.color = isAuto ? '#fff' : '#ccc';
    }
  }

  settingsGroup.appendChild(autoScaleBtn);
  toolbar.appendChild(settingsGroup);

  document.body.appendChild(toolbar);

  // --- Moving Average Control Panel ---
  const maPanel = document.createElement('div');
  Object.assign(maPanel.style, {
    position: 'absolute',
    top: '110px',
    left: '10px',
    padding: '10px',
    background: 'rgba(30, 30, 30, 0.9)',
    border: '1px solid #444',
    borderRadius: '4px',
    color: '#ccc',
    fontFamily: 'sans-serif',
    fontSize: '12px',
    display: 'flex',
    flexDirection: 'column',
    gap: '8px',
    zIndex: '100'
  });

  document.body.appendChild(maPanel);


  // --- Multi-MA & BB Config State ---
  let activeMaId = null;
  let maList = []; // Array of { id, settings }

  // BB Config
  // BB Config (Global JS State - needs to sync with Rust)
  let bbConfig = {
    period: 20,
    multiplier: 2.0,
    source: "Close",
    color: "#00ccff", // Cyan
    active: true,
    visible: true
  };

  // Sync JS State from Rust
  function syncBBUI() {
    const settings = engine.get_active_bb_settings();
    if (settings) {
      // Rust returns object with fields
      // Note: Rust 'source' is string "Close", etc.
      bbConfig = settings;
      // Trigger UI Re-render
      renderBBForm();
    }
  }

  function updateBB() {
    const cfg = { ...bbConfig };
    cfg.period = Math.round(cfg.period);
    engine.update_bb_settings(0, cfg);
  }

  // Reload MAs from Rust
  function refreshMAList() {
    maList = engine.get_all_mas(); // Returns [{id, settings}, ...]
    renderMATabs();

    // If active ID is no longer in list, default to first or null
    if (activeMaId !== null) {
      const exists = maList.find(m => m.id === activeMaId);
      if (!exists) {
        activeMaId = maList.length > 0 ? maList[0].id : null;
        updateMAForm();
      }
    } else if (maList.length > 0) {
      activeMaId = maList[0].id;
      updateMAForm();
    } else {
      // No MAs
      updateMAForm();
    }
  }

  // createMAInput helper (reused but slightly modified for dynamic val updates)
  // We need to clear the panel content each time or manage references.
  // Strategy: Clear form container -> Re-render inputs.

  const tabsContainer = document.createElement('div');
  tabsContainer.style.display = 'flex';
  tabsContainer.style.flexWrap = 'wrap';
  tabsContainer.style.gap = '5px';
  tabsContainer.style.marginBottom = '10px';
  maPanel.appendChild(tabsContainer);

  const formContainer = document.createElement('div');
  maPanel.appendChild(formContainer);

  // Render Tabs
  function renderMATabs() {
    tabsContainer.innerHTML = '';

    // Add Button
    const addBtn = document.createElement('button');
    addBtn.innerText = "+ Add MA";
    addBtn.style.padding = '3px 8px';
    addBtn.style.background = '#444';
    addBtn.style.color = '#fff';
    addBtn.style.border = '1px solid #555';
    addBtn.style.cursor = 'pointer';
    addBtn.style.fontSize = '10px';
    addBtn.onclick = () => {
      const newId = engine.add_ma();
      refreshMAList();
      activeMaId = newId;
      updateMAForm();
      renderMATabs(); // Re-render to highlight new tab
    };
    tabsContainer.appendChild(addBtn);

    maList.forEach((ma, idx) => {
      const btn = document.createElement('button');
      btn.innerText = `MA ${idx + 1}`;
      btn.style.padding = '3px 6px';
      btn.style.cursor = 'pointer';
      btn.style.fontSize = '10px';
      btn.style.border = '1px solid #555';

      if (ma.id === activeMaId) {
        btn.style.background = '#007bff';
        btn.style.color = '#fff';
      } else {
        btn.style.background = 'transparent';
        btn.style.color = '#ccc';
      }

      btn.onclick = () => {
        activeMaId = ma.id;
        renderMATabs();
        updateMAForm();
      };

      tabsContainer.appendChild(btn);
    });
  }

  // Render Form for Active MA
  function updateMAForm() {
    formContainer.innerHTML = '';

    const activeMA = maList.find(m => m.id === activeMaId);

    if (!activeMA) {
      formContainer.innerText = "No Active MA Selected.";
      formContainer.style.color = '#777';
      return;
    }

    const set = activeMA.settings;

    // Helper specific to this form
    function addInput(label, type, val, onChange) {
      createMAInput(formContainer, label, type, val, onChange);
    }

    // Title
    const title = document.createElement('div');
    title.innerText = `Edit MA (ID: ${activeMA.id})`;
    title.style.borderBottom = '1px solid #444';
    title.style.marginBottom = '5px';
    title.style.paddingBottom = '3px';
    title.style.fontWeight = 'bold';

    // Delete Button (Right aligned)
    const delBtn = document.createElement('button');
    delBtn.innerText = 'Del';
    delBtn.style.float = 'right';
    delBtn.style.fontSize = '10px';
    delBtn.style.background = '#d9534f';
    delBtn.style.color = '#fff';
    delBtn.style.border = 'none';
    delBtn.style.padding = '2px 5px';
    delBtn.style.cursor = 'pointer';
    delBtn.onclick = () => {
      engine.remove_ma(activeMaId);
      activeMaId = null;
      refreshMAList();
    };
    title.appendChild(delBtn);

    formContainer.appendChild(title);

    // Inputs
    // Inputs
    const commit = (updates) => {
      const newSettings = { ...activeMA.settings, ...updates };
      // Optimistic update local list
      activeMA.settings = newSettings;
      // Send to Rust
      engine.update_ma(activeMaId, newSettings);
    };

    addInput('Active', 'checkbox', set.visible, (v) => commit({ visible: v }));
    addInput('Type', 'select', { current: set.method === 0 ? 'SMA' : 'EMA', options: ['SMA', 'EMA'] }, (v) => commit({ method: v === 'SMA' ? 0 : 1 }));
    addInput('Period', 'number', set.period, (v) => commit({ period: Math.round(v) }));
    addInput('Source', 'select', { current: ['Close', 'Open', 'High', 'Low', 'HL2', 'HLC3'][set.source], options: ['Close', 'Open', 'High', 'Low', 'HL2', 'HLC3'] }, (v) => {
      const map = { 'Close': 0, 'Open': 1, 'High': 2, 'Low': 3, 'HL2': 4, 'HLC3': 5 };
      commit({ source: map[v] });
    });
    addInput('Offset', 'range_int', set.offset, (v) => commit({ offset: Math.round(v) }));
    addInput('Width', 'range', set.line_width, (v) => commit({ line_width: v }));
    addInput('Color', 'color', set.color, (v) => commit({ color: v }));
  }

  // Refactored createMAInput to accept parent
  function createMAInput(parent, label, type, val, onChange) {
    const row = document.createElement('div');
    row.style.display = 'flex';
    row.style.justifyContent = 'space-between';
    row.style.alignItems = 'center';
    row.style.marginBottom = '4px';

    const lbl = document.createElement('label');
    lbl.innerText = label;
    lbl.style.marginRight = '10px';

    let inp;
    if (type === 'select') {
      inp = document.createElement('select');
      inp.style.background = '#333';
      inp.style.color = '#ccc';
      inp.style.border = '1px solid #555';
      val.options.forEach(opt => {
        const o = document.createElement('option');
        o.value = opt;
        o.innerText = opt;
        inp.appendChild(o);
      });
      inp.value = val.current;
    } else {
      inp = document.createElement('input');
      inp.type = type === 'range_int' ? 'range' : type;

      if (type === 'checkbox') inp.checked = val;
      else inp.value = val;

      if (type === 'range' || type === 'range_int') {
        if (type === 'range_int') {
          inp.min = -50; inp.max = 50; inp.step = 1;
        } else {
          inp.min = 0.5; inp.max = 10; inp.step = 0.5;
        }
        inp.style.width = '60px';
      }
      if (type === 'number') {
        inp.style.width = '50px';
        inp.style.background = '#333';
        inp.style.color = '#ccc';
        inp.style.border = '1px solid #555';
      }
    }

    let valSpan = null;
    if (type === 'range' || type === 'range_int') {
      valSpan = document.createElement('span');
      valSpan.style.fontSize = '10px';
      valSpan.style.width = '25px';
      valSpan.style.textAlign = 'right';
      valSpan.innerText = val;
    }

    const handler = (e) => {
      let v;
      if (type === 'checkbox') v = e.target.checked;
      else if (type === 'range' || type === 'number' || type === 'range_int') v = parseFloat(e.target.value) || 0;
      else v = e.target.value;

      if (valSpan) valSpan.innerText = v;
      onChange(v);
    };

    inp.onchange = handler;
    if (type.includes('range') || type === 'color') inp.oninput = handler;

    row.appendChild(lbl);
    row.appendChild(inp);
    if (valSpan) row.appendChild(valSpan);
    parent.appendChild(row);
  }

  // --- Bollinger Band Config ---
  // Create Separator
  const sep = document.createElement('div');
  sep.style.height = '1px';
  sep.style.background = '#555';
  sep.style.margin = '10px 0';
  maPanel.appendChild(sep);

  const titleBB = document.createElement('div');
  titleBB.innerHTML = '<b>Bollinger Bands</b>';
  titleBB.style.borderBottom = '1px solid #555';
  titleBB.style.marginBottom = '5px';
  titleBB.style.paddingBottom = '3px';
  maPanel.appendChild(titleBB);

  const bbContainer = document.createElement('div');
  maPanel.appendChild(bbContainer);

  function renderBBForm() {
    bbContainer.innerHTML = '';
    // Re-use createMAInput but passing bbContainer
    createMAInput(bbContainer, 'Active', 'checkbox', bbConfig.visible, (v) => { bbConfig.visible = v; updateBB(); });
    createMAInput(bbContainer, 'Period', 'number', bbConfig.period, (v) => { bbConfig.period = v; updateBB(); });
    createMAInput(bbContainer, 'Mult', 'range', bbConfig.multiplier, (v) => { bbConfig.multiplier = v; updateBB(); });
    createMAInput(bbContainer, 'Source', 'select', { current: 'Close', options: ['Close', 'Open', 'High', 'Low', 'HL2', 'HLC3'] }, (v) => { bbConfig.source = v; updateBB(); });
    createMAInput(bbContainer, 'Color', 'color', bbConfig.color, (v) => { bbConfig.color = v; updateBB(); });
  }

  // Initial Load
  refreshMAList();
  if (maList.length === 0) {
    // Create initial default MA
    const newId = engine.add_ma();
    refreshMAList();
    activeMaId = newId;
    updateMAForm();
    renderMATabs();
  }
  renderBBForm();
  updateBB();

  // --- Keyboard Shortcuts ---
  window.addEventListener('keydown', (e) => {
    // Check for Cmd (Mac) or Ctrl (Windows/Linux)
    if (e.metaKey || e.ctrlKey) {
      if (e.key === 'z') {
        e.preventDefault(); // Prevent browser undo
        if (e.shiftKey) {
          engine.redo();
        } else {
          engine.undo();
        }
      }
    }

    // Deletion
    if (e.key === 'Backspace' || e.key === 'Delete') {
      engine.remove_selected_drawing();
    }
  });

  // Interaction State
  let isDragging = false;
  let isYAxisDragging = false; // New state for Y-Axis pan
  let lastX = 0;
  let lastY = 0;
  let mouseX = -1;
  let mouseY = -1;

  // Margins for Axis
  const MARGIN_RIGHT = 60;
  const MARGIN_BOTTOM = 30;

  // Click Listener for Object Selection (Drawings, MAs)
  canvas.addEventListener('click', (e) => {
    // Need precise coordinates relative to canvas
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Avoid conflict if we just finished dragging
    // (Simple way: if isDragging was true recently, ignore. But click fires after mouseup)
    // For now, let's assume if movement was minimal it's a click.

    // Check MA Hit
    const maId = engine.get_clicked_ma(x, y);
    if (maId !== undefined) {
      activeMaId = maId;
      updateMAForm();
      renderMATabs();
    }
  });

  let isAnchorDragging = false;

  // --- Drawing Property Editor ---
  const drawingEditor = document.createElement('div');
  Object.assign(drawingEditor.style, {
    position: 'absolute', display: 'none', zIndex: '200',
    background: '#222', border: '1px solid #555', padding: '8px',
    borderRadius: '4px', color: '#ccc', fontSize: '12px',
    boxShadow: '0 2px 8px rgba(0,0,0,0.5)'
  });
  document.body.appendChild(drawingEditor);

  function hideDrawingEditor() {
    drawingEditor.style.display = 'none';
  }

  function showDrawingEditor(x, y, props) {
    if (!props) return;

    // Position below MA Panel to prevent overlap
    let topPos = 350;
    if (typeof maPanel !== 'undefined') {
      const rect = maPanel.getBoundingClientRect();
      // If panel is populated, it has height. If empty/hidden, fallback.
      if (rect.height > 0) {
        topPos = rect.bottom + 10;
      }
    }

    drawingEditor.style.left = '10px';
    drawingEditor.style.top = topPos + 'px';
    drawingEditor.style.display = 'block';

    drawingEditor.innerHTML = ''; // Clear

    const title = document.createElement('div');
    title.innerText = 'Edit Drawing';
    title.style.fontWeight = 'bold';
    title.style.marginBottom = '5px';
    title.style.borderBottom = '1px solid #555';
    drawingEditor.appendChild(title);

    // Color
    createMAInput(drawingEditor, 'Color', 'color', props.color, (v) => {
      props.color = v;
      engine.update_selected_props(props);
    });

    // Width
    createMAInput(drawingEditor, 'Width', 'range', props.line_width, (v) => {
      props.line_width = v;
      engine.update_selected_props(props);
    });

    // Delete
    const delBtn = document.createElement('button');
    delBtn.innerText = 'Delete Drawing';
    delBtn.style.marginTop = '8px';
    delBtn.style.width = '100%';
    delBtn.style.background = '#d9534f';
    delBtn.style.color = 'white';
    delBtn.style.border = 'none';
    delBtn.style.padding = '4px';
    delBtn.style.cursor = 'pointer';
    delBtn.onclick = () => {
      engine.remove_selected_drawing();
      hideDrawingEditor();
    };
    drawingEditor.appendChild(delBtn);
  }

  canvas.addEventListener('mousedown', (e) => {
    const x = e.offsetX;
    const y = e.offsetY;
    const width = canvas.offsetWidth; // Use offsetWidth/Height for logical check
    const height = canvas.offsetHeight;
    const chartW = width - MARGIN_RIGHT;

    // 0. Y-Axis Drag Check (Right Margin)
    if (x > chartW) {
      isYAxisDragging = true;
      lastY = y;
      return;
    }

    // 1. Drawing Mode
    if (currentToolType >= 0) {
      if (isDrawingActive) {
        engine.update_drawing(x, y); // Snap to final click position
        engine.complete_drawing();
        isDrawingActive = false;
        currentToolType = -1;
        toolButtons.forEach(b => {
          b.style.background = 'transparent';
          b.style.borderColor = 'transparent';
          b.style.color = '#ccc';
        });
      } else {
        engine.start_drawing(currentToolType, x, y);
        isDrawingActive = true;
      }
      return;
    }

    // 2. Anchor Drag Check (Priority over Selection/Pan)
    if (engine.start_drag_anchor(x, y)) {
      isAnchorDragging = true;
      // Hide editor while dragging anchor? Maybe keep it.
      return;
    }

    // 3. Selection Check
    const hit = engine.check_click(x, y);
    if (hit) {
      console.log("Drawing Selected!");
      const props = engine.get_selected_props();
      showDrawingEditor(e.clientX, e.clientY, props);
      return;
    } else {
      hideDrawingEditor();
    }

    // 4. Pan Start
    isDragging = true;
    lastX = e.offsetX;
    lastY = e.offsetY;
  });

  canvas.addEventListener('mouseup', () => {
    if (isAnchorDragging) {
      engine.end_drag_anchor();
      isAnchorDragging = false;
    }
    isDragging = false;
    isYAxisDragging = false;

    // Check auto scale status (drag might have disabled it)
    updateAutoScaleUI();
  });

  canvas.addEventListener('mouseleave', () => {
    isDragging = false;
    isYAxisDragging = false;
    isAnchorDragging = false;
    engine.end_drag_anchor();
    mouseX = -1;
    mouseY = -1;
    updateAutoScaleUI();
  });

  canvas.addEventListener('mousemove', (e) => {
    mouseX = e.offsetX;
    mouseY = e.offsetY;

    // Handle Anchor Dragging
    if (isAnchorDragging) {
      engine.update_drag_anchor(mouseX, mouseY);
      return; // Don't allow other interactions while dragging anchor
    }

    // Handle Y-Axis Pan (Right Margin Drag)
    if (isYAxisDragging) {
      const dy = e.offsetY - lastY;
      engine.pan_y(dy);
      lastY = e.offsetY;
      updateAutoScaleUI();
      return;
    }

    // Auto-start for Horizontal (3) and Vertical (4)
    if ((currentToolType === 3 || currentToolType === 4) && !isDrawingActive) {
      engine.start_drawing(currentToolType, mouseX, mouseY);
      isDrawingActive = true;
    }

    // Handle Drawing Update
    if (isDrawingActive) {
      engine.update_drawing(mouseX, mouseY);
    }

    // Handle Panning
    if (isDragging) {
      const dx = e.offsetX - lastX;
      engine.pan(dx);

      // If Manual Mode, also Pan Y
      if (!engine.get_auto_scale()) {
        const dy = e.offsetY - lastY;
        engine.pan_y(dy);
      }

      lastX = e.offsetX;
      lastY = e.offsetY; // Update lastY too!
    }
  });

  canvas.addEventListener('wheel', (e) => {
    e.preventDefault();

    // 1. Horizontal Scroll (Pan X) - Trackpad Intent
    // Check if horizontal movement dominates vertical (with some bias to X to prevent accidental zooming)
    // Using 0.5 threshold means if Horizontal is at least half of Vertical, we treat it as Horizontal.
    if (Math.abs(e.deltaX) > Math.abs(e.deltaY) * 0.5) {
      const sensitivity = 0.5;
      // Invert deltaX for natural feel (Swipe Left -> Pan Right)
      engine.pan(-e.deltaX * sensitivity);

      // Block zoom function during horizontal swipe
      return;
    }

    // 2. Vertical Logic (Zooming)
    if (e.metaKey || e.ctrlKey) {
      // Cmd + Wheel = Zoom Y (Price Axis)
      const isZoomOut = e.deltaY > 0;
      const factor = isZoomOut ? 1.1 : 0.9;
      engine.zoom_y(factor);
      updateAutoScaleUI();
    } else {
      // Regular Wheel (Vertical Swipe) = Zoom X (Time Axis)
      // We want low sensitivity.
      // E.g. 0.1% change per pixel delta?
      // deltaY > 0 -> Zoom Out (factor < 1.0) ? 
      // Typcially Wheel Down (positive) -> Zoom Out (view more).

      const sensitivity = 0.001;
      // Limit factor to avoid explosion
      let zoomDelta = e.deltaY * sensitivity;

      // Clamp logic if needed, but linear small factor is usually fine.
      // factor = 1.0 + zoomDelta ??
      // If deltaY = 10 (swipe down), factor = 1.01? Wait.
      // Zoom Out means factor < 1.0? 
      // In Rust: new_scale = old * factor.
      // Zoom Out -> Scale get SMALLER (more candles).
      // So deltaY > 0 -> factor < 1.0.

      const factor = 1.0 - zoomDelta;

      engine.zoom(factor, mouseX);
    }
  }, { passive: false });

  let isLoading = false;
  let noMoreHistory = false;

  async function fetchOlderData() {
    if (isLoading || noMoreHistory) return;

    const flatData = chartDataCache.get(activeChartId);
    const oldestTime = flatData ? flatData[0] : null;
    if (!oldestTime) return;

    console.log('Fetching older data ending at:', new Date(oldestTime).toLocaleString());
    isLoading = true;

    try {
      const info = engine.get_tabs_info();
      const activeTab = info.find(t => t.id === activeChartId);
      const interval = activeTab ? activeTab.interval : '1d';

      const res = await fetch(`https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=${interval}&limit=1000&endTime=${oldestTime - 1}`);
      const data = await res.json();

      if (!data || data.length === 0) {
        console.log('No more history.');
        noMoreHistory = true;
        return;
      }

      // Parse New Data
      const newFlat = new Float64Array(data.length * 6);
      for (let i = 0; i < data.length; i++) {
        const k = data[i];
        const offset = i * 6;
        newFlat[offset] = Number(k[0]);
        newFlat[offset + 1] = Number(k[1]);
        newFlat[offset + 2] = Number(k[2]);
        newFlat[offset + 3] = Number(k[3]);
        newFlat[offset + 4] = Number(k[4]);
        newFlat[offset + 5] = Number(k[5]);
      }

      const combined = new Float64Array(newFlat.length + flatData.length);
      combined.set(newFlat);
      combined.set(flatData, newFlat.length);
      chartDataCache.set(activeChartId, combined);

      engine.prepend_candles(activeChartId, newFlat);
      console.log(`Loaded ${data.length} older candles.`);

    } catch (e) {
      console.error('Error fetching older data:', e);
    } finally {
      isLoading = false;
    }
  }

  function render() {
    const bufferPtr = engine.get_render_buffer_ptr();
    const bufferLen = engine.get_render_buffer_len();

    const viewState = engine.get_view_state();
    const minPrice = viewState[0];
    const maxPrice = viewState[1];
    const startIdx = viewState[2];
    const endIdx = viewState[3];

    const globalFlatData = chartDataCache.get(activeChartId);
    // Check for Infinite Scroll
    if (startIdx < 50 && !isLoading && !noMoreHistory && !isTimeframeChanging && globalFlatData && globalFlatData.length > 0) {
      fetchOlderData();
    }

    const data = new Float32Array(wasm.memory.buffer, bufferPtr, bufferLen);

    const width = canvas.width / dpr;
    const height = canvas.height / dpr;
    const chartW = width - MARGIN_RIGHT;
    const chartH = height - MARGIN_BOTTOM;

    // 3. Clear Canvas
    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = COLORS.background;
    ctx.fillRect(0, 0, width, height);

    // --- Draw Chart Area ---
    // We don't translate context, engine coords are already 0 based.

    // 4. Draw Candlesticks & Volume
    let candleWidth = 5;
    if (bufferLen >= 12) { // Need at least 2 candles (2 * 6 = 12 floats)
      candleWidth = (data[6] - data[0]) * 0.8;
    }

    // Clip Chart Area
    ctx.save();
    ctx.beginPath();
    ctx.rect(0, 0, chartW, chartH);
    ctx.clip();

    ctx.lineWidth = 1;

    for (let i = 0; i < bufferLen; i += 6) {
      const x = data[i];
      const h = data[i + 1];
      const l = data[i + 2];
      const o = data[i + 3];
      const c = data[i + 4];
      const vTop = data[i + 5];

      const isUp = c < o;

      const color = isUp ? COLORS.candleUp : COLORS.candleDown;
      ctx.strokeStyle = color;
      ctx.fillStyle = color;

      // Draw Wick
      ctx.beginPath();
      ctx.moveTo(x, h);
      ctx.lineTo(x, l);
      ctx.stroke();

      // Draw Body
      const bodyH = Math.max(1, Math.abs(o - c));
      const bodyY = Math.min(o, c);
      ctx.fillRect(x - candleWidth / 2, bodyY, candleWidth, bodyH);

      // Draw Volume Bar
      ctx.globalAlpha = 0.5;
      const volH = Math.max(0, chartH - vTop);
      ctx.fillRect(x - candleWidth / 2, vTop, candleWidth, volH);
      ctx.globalAlpha = 1.0;
    }



    // --- Render Bollinger Bands ---
    // Must be before MA to be BEHIND them? Or after? Usually behind candles?
    // Let's render BB BEHIND MAs.
    // Actually, drawing behind Candlesticks might be better for visibility?
    // But implementation order is here. Let's do it here (on top of candles, behind drawings).

    // NOTE: If we want BB behind candles, we should move this block up.
    // For now, let's keep it here.

    const bbCount = engine.get_bb_count();
    for (let i = 0; i < bbCount; i++) {
      const bbLen = engine.get_bb_buffer_len(i);
      if (bbLen === 0) continue;

      const bbPtr = engine.get_bb_buffer_ptr(i);
      const bbColor = engine.get_bb_color(i);

      const bbData = new Float32Array(wasm.memory.buffer, bbPtr, bbLen);

      // Fill Area
      ctx.fillStyle = bbColor + "1A"; // Hex + Alpha (approx 10%)
      // Or better: parse hex and rgba. "1A" = 26/255 ~= 0.1

      ctx.beginPath();
      if (bbLen >= 2) {
        ctx.moveTo(bbData[0], bbData[1]);
        for (let j = 2; j < bbLen; j += 2) {
          ctx.lineTo(bbData[j], bbData[j + 1]);
        }
        ctx.closePath();
        ctx.fill();

        // Optional: Stroke Edges (Upper and Lower)
        // The buffer is Upper(L->R) + Lower(R->L).
        // We can stroke the outline.
        ctx.strokeStyle = bbColor;
        ctx.lineWidth = 1;
        ctx.stroke();
      }
    }

    // --- Render Moving Averages ---
    const maCount = engine.get_ma_count();
    for (let i = 0; i < maCount; i++) {
      const maLen = engine.get_ma_buffer_len(i);
      if (maLen === 0) continue;

      const maPtr = engine.get_ma_buffer_ptr(i);
      const maColor = engine.get_ma_color(i);
      const maWidth = engine.get_ma_width(i);

      const maData = new Float32Array(wasm.memory.buffer, maPtr, maLen);

      ctx.strokeStyle = maColor;
      ctx.lineWidth = maWidth;
      ctx.beginPath();

      // Command format: 1.0 (Move), x, y | 2.0 (Line), x, y
      for (let j = 0; j < maLen; j += 3) {
        const cmd = maData[j];
        const x = maData[j + 1];
        const y = maData[j + 2];

        if (cmd === 1.0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
      }
      ctx.stroke();
    }

    // --- Render Overlay Drawings ---
    const drawPtr = engine.get_drawing_buffer_ptr();
    const drawLen = engine.get_drawing_buffer_len();
    if (drawLen > 0) {
      const drawData = new Float32Array(wasm.memory.buffer, drawPtr, drawLen);

      ctx.strokeStyle = COLORS.drawing; // Cyan for drawings
      ctx.lineWidth = 2;
      ctx.beginPath();

      let i = 0;
      while (i < drawLen) {
        const cmd = drawData[i];

        if (cmd === 4.0) { // Style Change: [4.0, width, r, g, b]
          // Flush previous batch
          ctx.stroke();

          const w = drawData[i + 1];
          const r = drawData[i + 2];
          const g = drawData[i + 3];
          const b = drawData[i + 4];

          ctx.lineWidth = w;
          ctx.strokeStyle = `rgb(${r},${g},${b})`;
          ctx.beginPath();
          i += 5;
          continue;
        }

        const x = drawData[i + 1];
        const y = drawData[i + 2];

        if (cmd === 1.0) { // MoveTo
          ctx.moveTo(x, y);
          i += 3;
        } else if (cmd === 2.0) { // LineTo
          ctx.lineTo(x, y);
          i += 3;
        } else if (cmd === 3.0) { // Circle (Anchor) [3.0, x, y, r, g, b]
          // Draw the current path so far
          ctx.stroke();
          ctx.beginPath();

          const r = drawData[i + 3];
          const g = drawData[i + 4];
          const b = drawData[i + 5];

          // Draw Anchor
          ctx.save();
          ctx.fillStyle = COLORS.anchorFill;
          ctx.strokeStyle = `rgb(${r},${g},${b})`;
          ctx.lineWidth = 2;
          ctx.beginPath();
          ctx.arc(x, y, 4, 0, Math.PI * 2);
          ctx.fill();
          ctx.stroke();
          ctx.restore();

          // Resume path
          ctx.beginPath();
          ctx.moveTo(x, y);

          i += 6;
        } else {
          i += 1; // Fallback to avoid infinite loop
        }
      }
      ctx.stroke(); // Final Flush
    }

    ctx.restore();

    // --- Draw Axes ---
    ctx.font = '10px sans-serif';
    ctx.fillStyle = COLORS.axisText;
    ctx.strokeStyle = COLORS.grid;

    // Y-Axis (Price)
    if (minPrice < maxPrice) {
      const range = maxPrice - minPrice;
      const steps = 5;
      const paddingY = 20; // Matches Rust's padding
      const availH = chartH - (2 * paddingY);

      for (let i = 0; i <= steps; i++) {
        const ratio = i / steps;
        const price = minPrice + (range * ratio);
        const y = chartH - paddingY - (ratio * availH);

        // Draw Grid Line
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(chartW, y);
        ctx.stroke();

        // Draw Label
        ctx.fillText(price.toFixed(2), chartW + 5, y + 3);
      }
    }

    // X-Axis (Time)
    if (globalFlatData && startIdx < endIdx) {
      // Alternative X-Axis: Just pick 5 equidistant points from the render buffer
      if (bufferLen > 0) {
        const numCandles = bufferLen / 6; // Stride 6
        const labelStep = Math.max(1, Math.floor(numCandles / 5));

        for (let i = 0; i < numCandles; i += labelStep) {
          const offset = i * 6; // Stride 6
          const x = data[offset];

          // Find timestamp
          const realIdx = startIdx + i;
          if (realIdx * 6 < globalFlatData.length) { // Stride 6 checking
            const ts = globalFlatData[realIdx * 6];  // Stride 6 checking
            const dateStr = new Date(ts).toLocaleDateString();

            ctx.fillText(dateStr, x, chartH + 15);

            // Grid
            ctx.beginPath();
            ctx.moveTo(x, 0);
            ctx.lineTo(x, chartH);
            ctx.stroke();
          }
        }
      }
    }

    // 5. Draw Crosshair
    if (mouseX >= 0 && mouseY >= 0 && mouseX < chartW && mouseY < chartH) {
      ctx.strokeStyle = COLORS.crosshair;
      ctx.setLineDash([5, 5]);
      ctx.lineWidth = 1;

      ctx.beginPath();
      ctx.moveTo(mouseX, 0);
      ctx.lineTo(mouseX, chartH);
      ctx.stroke();

      ctx.beginPath();
      ctx.moveTo(0, mouseY);
      ctx.lineTo(chartW, mouseY);
      ctx.stroke();

      ctx.setLineDash([]);

      // Draw Crosshair Labels
      // Price
      const paddingY = 20;
      const availH = chartH - (2 * paddingY);
      if (availH > 0) {
        const normY = (chartH - paddingY - mouseY) / availH;
        const hoverPrice = minPrice + (normY * (maxPrice - minPrice));

        ctx.fillStyle = COLORS.crosshairLabel;
        ctx.fillText(hoverPrice.toFixed(2), chartW + 5, mouseY + 3);
      }
    }

    requestAnimationFrame(render);
  }

  // Debug info
  // console.log("Starting App...");
  // console.log("Rust Tabs Info:", engine.get_tabs_info());
  // console.log("JS ActiveChartId:", activeChartId);

  loadData(1, '1d');
  render();

  // Simulated WebSocket Feed
  setInterval(() => {
    let globalFlatData = chartDataCache.get(activeChartId);
    if (globalFlatData && globalFlatData.length > 5) {
      // Modify last candle close (Index: len - 2)
      // Stride 6: time, open, high, low, close, vol
      const lastCloseIdx = globalFlatData.length - 2;
      const lastHighIdx = globalFlatData.length - 4;
      const lastLowIdx = globalFlatData.length - 3;

      const currentPrice = globalFlatData[lastCloseIdx];
      // Random walk step
      const change = (Math.random() - 0.5) * 10;
      const newPrice = currentPrice + change;

      // Update JS State (so next tick is continuous)
      globalFlatData[lastCloseIdx] = newPrice;

      // Simple High/Low adjustment for visual coherence
      if (newPrice > globalFlatData[lastHighIdx]) globalFlatData[lastHighIdx] = newPrice;
      if (newPrice < globalFlatData[lastLowIdx]) globalFlatData[lastLowIdx] = newPrice;

      // Send to Rust
      // We only update Active Chart for this demo because we only track globalFlatData for active.
      // Real WS would have { chartId: price } messages.
      engine.update_last_candle(activeChartId, newPrice);
    }
  }, 1000);
}

run();