<template>
  <div style="height: 100%; width: 100%; background: #002833">
    <div id="terminal" ref="terminal" class="terminal-container"></div>
  </div>
</template>

<script setup>
import { onMounted, onScopeDispose, ref } from 'vue'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import { WebLinksAddon } from 'xterm-addon-web-links'
import { SearchAddon } from 'xterm-addon-search'
import 'xterm/css/xterm.css'
import { io } from 'socket.io-client'

const props = defineProps({
  url: String,
})

const terminal = ref(null)
const term = new Terminal({
  rendererType: 'canvas',
  convertEol: true,
  disableStdin: false,
  cursorBlink: true,
  scrollback: 0,
  theme: {
    fontFamily: 'Consolas, "Courier New", monospace',
    foreground: 'wihite',
    background: '#002833',
    cursor: 'help',
    lineHeight: 16,
  },
})
const fitAddon = new FitAddon()
const searchAddon = new SearchAddon()
const socket = io(props.url)

const resizeScreen = () => {
  fitAddon.fit()
  if (term) {
    const windowSize = { height: term.rows, width: term.cols }
    socket.emit('resize', windowSize)
  }
}

const initSocket = () => {
  socket.on('connect', () => {
    console.log('connect')
    resizeScreen()
  })

  socket.on('connected', data => {
    console.log('connected')
    if (data == 'ok') {
      resizeScreen()
    }
  })

  socket.on('out', data => {
    term.write(data)
  })

  socket.on('err', data => {
    if (data) {
      term.write(data + '\n')
    }
  })
}

const initTerm = () => {
  term.loadAddon(new WebLinksAddon())
  term.loadAddon(searchAddon)
  term.loadAddon(fitAddon)
  term.open(terminal.value)
  term.focus()
  fitAddon.fit()

  term.onData(data => {
    socket.emit('data', data)
  })
}

onMounted(() => {
  initSocket()
  initTerm()
  window.addEventListener('resize', resizeScreen, false)
})

onScopeDispose(() => {
  if (term) {
    term.dispose()
  }
  if (socket) {
    socket.disconnect()
  }
  window.removeEventListener('resize', resizeScreen, false)
})
</script>

<style>
.terminal-container {
  display: block;
  width: calc(100% - 1px);
  margin: 0 auto;
  padding: 2px;
  height: calc(100% - 19px);
}
.terminal-container .terminal {
  background-color: #000000;
  color: #fafafa;
  padding: 2px;
  height: calc(100% - 19px);
}
.terminal-container .terminal:focus .terminal-cursor {
  background-color: #fafafa;
}
</style>
