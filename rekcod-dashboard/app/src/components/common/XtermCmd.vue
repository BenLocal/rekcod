<template>
  <div style="height: 100%; background: #002833">
    <div id="terminal" ref="terminal"></div>
  </div>
</template>

<script setup>
import { onMounted, onScopeDispose, ref } from 'vue'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import { WebLinksAddon } from 'xterm-addon-web-links'
import { SearchAddon } from 'xterm-addon-search'
import 'xterm/css/xterm.css'

// const props = defineProps({
//   remote: String,
// })

const terminal = ref(null)
const termOptions = ref({
  rows: 30,
  cols: 80,
})
let term = null

const initTerm = () => {
  term = new Terminal({
    // 渲染类型
    rendererType: 'canvas',
    // 行数
    rows: parseInt(termOptions.value.rows),
    // 不指定行数，自动回车后光标从下一行开始
    cols: parseInt(termOptions.value.cols),
    // 启用时，光标将设置为下一行的开头
    convertEol: true,
    // 终端中的回滚量
    scrollback: 50,
    // 是否应禁用输入。
    disableStdin: false,
    // 光标样式
    cursorStyle: 'underline',
    // 光标闪烁
    cursorBlink: true,
    theme: {
      // 字体
      fontFamily: 'Consolas, "Courier New", monospace',
      // 字体颜色
      foreground: 'wihite',
      // 背景色
      background: '#002833',
      // 设置光标
      cursor: 'help',
      lineHeight: 16,
    },
  })

  const fitAddon = new FitAddon()
  const searchAddon = new SearchAddon()
  term.loadAddon(new WebLinksAddon())
  term.loadAddon(searchAddon)
  term.open(terminal.value)
  fitAddon.fit()
  term.prompt = () => {
    term.write('\r$ ')
  }
  term.prompt()

  term.onKey(e => {
    const printable =
      !e.domEvent.altKey &&
      !e.domEvent.altGraphKey &&
      !e.domEvent.ctrlKey &&
      !e.domEvent.metaKey
    if (e.domEvent.key === 'Enter') {
      // TODO
    } else if (e.domEvent.key === 'Backspace') {
      if (term._core.buffer.x > 2) {
        term.write('\b \b')
      }
    } else if (printable) {
      term.write(e.key)
      // TODO
    } else {
      // TODO
    }
  })
}

onMounted(() => {
  initTerm()
})

onScopeDispose(() => {
  if (term) {
    term.dispose()
  }
})
</script>
