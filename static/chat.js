(function () {
  const messagesEl = document.getElementById("messages");
  const inputEl = document.getElementById("input");
  const sendBtn = document.getElementById("send");
  const statusEl = document.getElementById("status");

  let ws = null;
  let reconnectTimer = null;
  let thinking = false;

  function connect() {
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    ws = new WebSocket(`${proto}//${location.host}/ws`);

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
      let data;
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
    const el = document.createElement("div");
    el.className = "message " + type;
    el.textContent = text;
    messagesEl.appendChild(el);
    scrollToBottom();
    enableInput();
  }

  function showThinking() {
    thinking = true;
    disableInput();
    let el = document.getElementById("thinking");
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
    const el = document.getElementById("thinking");
    if (el) el.remove();
  }

  function sendMessage() {
    const text = inputEl.value.trim();
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

  // Start connection
  connect();
})();
