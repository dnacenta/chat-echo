(function () {
  const messagesEl = document.getElementById("messages");
  const inputEl = document.getElementById("input");
  const sendBtn = document.getElementById("send");
  const statusEl = document.getElementById("status");
  const dashboardEl = document.getElementById("dashboard");

  let ws = null;
  let reconnectTimer = null;
  let thinking = false;

  // --- Dashboard Banner ---

  function fetchDashboard() {
    const base = location.pathname.replace(/\/+$/, '');
    fetch(base + '/api/dashboard')
      .then(function (res) {
        if (!res.ok) throw new Error(res.status);
        return res.json();
      })
      .then(renderDashboard)
      .catch(function () {
        // Dashboard not available — hide the block
        dashboardEl.style.display = 'none';
      });
  }

  function renderDashboard(data) {
    if (!data || !data.entity) {
      dashboardEl.style.display = 'none';
      return;
    }

    var lines = [];

    // ASCII logo + metadata
    var logo = [
      '  \u2554\u2550\u2557\u2566 \u2566\u2566  \u2554\u2550\u2557\u2554\u2550\u2557   \u2554\u2557\u2554\u2566 \u2566\u2566  \u2566',
      '  \u2560\u2550\u2569\u2551 \u2551\u2551  \u255a\u2550\u2557\u2551\u2563    \u2551\u2551\u2551\u2551 \u2551\u2551  \u2551',
      '  \u2569  \u255a\u2550\u255d\u2569\u2550\u2569\u255a\u2550\u255d\u255a\u2550\u255d\u2500\u2500\u2500\u255d\u255a\u255d\u255a\u2550\u255d\u2569\u2550\u255d\u2569\u2550\u255d'
    ];

    var e = data.entity;
    var meta = [
      'entity  ' + esc(e.name),
      'user    ' + esc(e.user),
      'model   ' + esc(e.model),
      'plugins ' + (e.plugins && e.plugins.length > 0 ? e.plugins.length + ' active' : 'none')
    ];

    for (var i = 0; i < Math.max(logo.length, meta.length); i++) {
      var left = i < logo.length ? logo[i] : '';
      var right = i < meta.length ? meta[i] : '';
      // Pad logo column
      while (left.length < 38) left += ' ';
      lines.push(left + right);
    }

    lines.push('  v' + esc(e.version || ''));
    lines.push('  ' + '\u2500'.repeat(60));

    // Pipeline bars
    if (data.pipeline) {
      lines.push('');
      var docs = ['learning', 'thoughts', 'curiosity', 'reflections', 'praxis'];
      for (var d = 0; d < docs.length; d++) {
        var name = docs[d];
        var doc = data.pipeline[name];
        if (!doc) continue;
        lines.push(pipelineLine(name, doc));
      }
      if (data.pipeline.warnings && data.pipeline.warnings.length > 0) {
        for (var w = 0; w < data.pipeline.warnings.length; w++) {
          lines.push('  <span class="dash-warning">! ' + esc(data.pipeline.warnings[w]) + '</span>');
        }
      }
    }

    // Cognitive health
    if (data.cognitive_health) {
      lines.push('  ' + '\u2500'.repeat(60));
      var ch = data.cognitive_health;
      if (!ch.sufficient_data) {
        lines.push('  <span class="dash-dim">cognitive health  awaiting data</span>');
      } else {
        var statusClass = 'dash-' + ch.status;
        lines.push('  cognitive health  <span class="' + statusClass + '">' + esc(ch.status).toUpperCase() + '</span>');
        if (ch.signals) {
          var s = ch.signals;
          lines.push(
            '  vocabulary ' + trendArrow(s.vocabulary) +
            '  questions ' + trendArrow(s.questions) +
            '  grounding ' + trendArrow(s.grounding) +
            '  lifecycle ' + trendArrow(s.lifecycle)
          );
        }
      }
    }

    // Boot animation — type in line by line
    dashboardEl.style.display = '';
    var pre = document.createElement('pre');
    pre.className = 'dash-pre';
    dashboardEl.innerHTML = '';
    dashboardEl.appendChild(pre);
    animateLines(pre, lines, 0);
  }

  function animateLines(pre, lines, idx) {
    if (idx >= lines.length) return;
    if (idx > 0) pre.innerHTML += '\n';
    pre.innerHTML += lines[idx];
    scrollToBottom();
    setTimeout(function () { animateLines(pre, lines, idx + 1); }, 40);
  }

  function pipelineLine(name, doc) {
    var width = 10;
    var filled = doc.hard_limit > 0 ? Math.min(Math.floor(doc.count * width / doc.hard_limit), width) : 0;
    var empty = width - filled;
    var bar = '\u2588'.repeat(filled) + '\u2591'.repeat(empty);
    var count = doc.count + '/' + doc.hard_limit;
    var statusWord = doc.status === 'green' ? 'ok' : doc.status === 'yellow' ? 'warning' : 'full';
    var cls = 'dash-' + doc.status;

    // Pad name to 14 chars
    while (name.length < 14) name += ' ';
    // Pad count to 6 chars
    while (count.length < 6) count += ' ';

    return '  ' + esc(name) + ' <span class="' + cls + '">' + bar + '  ' + count + '</span>  ' + statusWord;
  }

  function trendArrow(trend) {
    if (trend === 'up') return '<span class="dash-green">\u25b2</span>';
    if (trend === 'down') return '<span class="dash-red">\u25bc</span>';
    return '\u2500';
  }

  function esc(s) {
    if (!s) return '';
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  // --- WebSocket Chat ---

  function connect() {
    var proto = location.protocol === "https:" ? "wss:" : "ws:";
    var base = location.pathname.replace(/\/+$/, '');
    ws = new WebSocket(proto + "//" + location.host + base + "/ws");

    ws.onopen = function () {
      setStatus("connected");
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
    };

    ws.onclose = function () {
      setStatus("disconnected");
      scheduleReconnect();
    };

    ws.onerror = function () {
      setStatus("disconnected");
    };

    ws.onmessage = function (event) {
      var data;
      try {
        data = JSON.parse(event.data);
      } catch (e) {
        return;
      }

      if (data.status === "thinking") {
        showThinking();
      } else if (data.response !== undefined) {
        hideThinking();
        addMessage(data.response, "ai");
      } else if (data.error) {
        hideThinking();
        addMessage(data.error, "error");
      }
    };
  }

  function scheduleReconnect() {
    if (reconnectTimer) return;
    setStatus("reconnecting");
    reconnectTimer = setTimeout(function () {
      reconnectTimer = null;
      connect();
    }, 3000);
  }

  function setStatus(state) {
    statusEl.className = state;
    if (state === "connected") {
      statusEl.innerHTML = '<span class="status-dot"></span>connected';
    } else if (state === "reconnecting") {
      statusEl.innerHTML = '<span class="status-dot"></span>reconnecting';
    } else {
      statusEl.innerHTML = '<span class="status-dot"></span>disconnected';
    }
  }

  function addMessage(text, type) {
    var el = document.createElement("div");
    el.className = "message " + type;
    el.textContent = text;
    messagesEl.appendChild(el);
    scrollToBottom();
    enableInput();
  }

  function showThinking() {
    thinking = true;
    disableInput();
    var el = document.getElementById("thinking");
    if (!el) {
      el = document.createElement("div");
      el.id = "thinking";
      el.className = "thinking";
      el.innerHTML = 'Thinking<span class="dots"></span>';
      messagesEl.appendChild(el);
    }
    scrollToBottom();
  }

  function hideThinking() {
    thinking = false;
    var el = document.getElementById("thinking");
    if (el) el.remove();
  }

  function sendMessage() {
    var text = inputEl.value.trim();
    if (!text || !ws || ws.readyState !== WebSocket.OPEN) return;

    addMessage(text, "user");
    ws.send(JSON.stringify({ message: text }));
    inputEl.value = "";
    inputEl.style.height = "auto";
  }

  function disableInput() {
    sendBtn.disabled = true;
    inputEl.disabled = true;
  }

  function enableInput() {
    sendBtn.disabled = false;
    inputEl.disabled = false;
    inputEl.focus();
  }

  function scrollToBottom() {
    messagesEl.scrollTop = messagesEl.scrollHeight;
  }

  // Auto-resize textarea
  inputEl.addEventListener("input", function () {
    this.style.height = "auto";
    this.style.height = Math.min(this.scrollHeight, 120) + "px";
  });

  // Enter to send, Shift+Enter for newline
  inputEl.addEventListener("keydown", function (e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  });

  sendBtn.addEventListener("click", sendMessage);

  // Start
  fetchDashboard();
  connect();
})();
