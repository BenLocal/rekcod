<template>
  <el-table :data="containerList" style="width: 100%">
    <el-table-column prop="id" label="id" width="180" />
    <el-table-column prop="name" label="名称" width="180" />
    <el-table-column prop="image" label="镜像" />
    <el-table-column prop="state" label="状态">
      <template #default="scope">
        <el-tag :type="scope.row.stateTag">
          {{ scope.row.state }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column prop="status" label="运行时长" />
    <el-table-column label="操作">
      <template #default="scope">
        <el-link
          :icon="CommandLineIcon"
          @click="attch_container(scope.row.id)"
        ></el-link>
        <el-link
          :icon="MoreFilled"
          @click="on_open_control(scope.row.id)"
        ></el-link>
      </template>
    </el-table-column>
  </el-table>

  <el-drawer
    v-model="openTerminal"
    direction="btt"
    with-header="false"
    size="70%"
  >
    <XtermCmd />
  </el-drawer>
  <el-drawer
    v-model="openControl"
    title="控制面板"
    :with-header="false"
    size="50%"
  >
    <el-row>
      <el-button type="danger" @click="stop_container">停止</el-button>
      <el-button type="primary" @click="start_container">开始</el-button>
      <el-button type="danger" @click="restart_container">重启</el-button>
      <el-button type="primary" @click="logs_container">日志</el-button>
      <el-button type="danger" @click="remove_container">删除</el-button>
      <el-button type="primary" @click="inspect_container">信息</el-button>
    </el-row>
    <el-row v-if="currentContainerInfo && currentContainerInfo != ''">
      <dev class="container-code">
        <Codemirror
          ref="codeMirrorInfoRef"
          :value="currentContainerInfo"
          :options="containerInfoOptions"
          border
        ></Codemirror>
      </dev>
    </el-row>
  </el-drawer>
  <el-dialog
    title="日志"
    v-model="logsState.logsDialog"
    :fullscreen="true"
    destroy-on-close="true"
    @closed="closeLogsDialog"
  >
    <dev class="container-code">
      <Codemirror
        ref="codeMirrorRef"
        :value="logsState.logsTxt"
        :options="logsState.txtOptions"
        border
      ></Codemirror>
    </dev>
    <template #footer>
      <el-button @click="closeLogsDialog">关闭</el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { onMounted, ref, shallowRef } from 'vue'
import api from '../../api'
import { ElMessage } from 'element-plus'
import Codemirror from 'codemirror-editor-vue3'
import { MoreFilled } from '@element-plus/icons-vue'
import { CommandLineIcon } from '@heroicons/vue/24/solid'
import XtermCmd from '../common/XtermCmd.vue'

const props = defineProps({
  node_name: String,
})

const containerList = ref([])
const openControl = ref(false)
const selectContainerId = ref('')
const logsState = ref({
  cancel: null,
  logsDialog: false,
  logsTxt: '',
  txtOptions: {},
})
const codeMirrorRef = shallowRef()
const logMaxlines = 1000
const currentContainerInfo = ref(null)
const codeMirrorInfoRef = shallowRef()
const containerInfoOptions = ref({
  mode: 'json',
  lineNumbers: true,
  autoRefresh: true,
  readonly: true,
  scrollbarStyle: 'native',
})
const openTerminal = ref(false)

const refresh = async () => {
  await get_docker_container_list(props.node_name)
}

const attch_container = () => {
  openTerminal.value = true
}

const inspect_container = async () => {
  const { code, data, msg } = await (
    await api.inspectDockerContainerByNode(
      props.node_name,
      selectContainerId.value,
    )
  ).data
  if (code !== 0) {
    ElMessage.error(msg || '获取容器信息失败')
    return
  }

  currentContainerInfo.value = JSON.stringify(data, null, 2)
}

const remove_container = async () => {
  const { code, msg } = await (
    await api.removeDockerContainerByNode(
      props.node_name,
      selectContainerId.value,
    )
  ).data
  if (code !== 0) {
    ElMessage.error(msg || '删除容器失败')
    return
  }

  ElMessage.success('删除容器成功')
  refresh()
}

const on_container_logs_chunk = chunk => {
  const codemirror = codeMirrorRef.value?.cminstance
  if (!codemirror) return

  //append to doc
  const doc = codemirror.getDoc()
  const currentLines = doc.lineCount()
  if (currentLines + 200 > logMaxlines) {
    const linesToRemove = currentLines + 200 - logMaxlines
    doc.replaceRange('', { line: 0, ch: 0 }, { line: linesToRemove, ch: 0 })
  }

  //const ansiUpchunk = ansiUp.ansi_to_html(chunk)
  doc.replaceRange(chunk, doc.posFromIndex(doc.getValue().length))

  const scroll = codemirror.getScrollInfo()
  if (!scroll) return
  codemirror.scrollTo(0, scroll.height)
}

const closeLogsDialog = () => {
  logsState.value.cancel && logsState.value.cancel.abort()
  logsState.value.cancel = null
  logsState.value.logsDialog = false
}

const logs_container = async () => {
  logsState.value = {
    cancel: new AbortController(),
    logsDialog: true,
    logsTxt: '',
    txtOptions: {
      mode: 'log',
      lineNumbers: true,
      autoRefresh: true,
      readonly: true,
      scrollbarStyle: 'native',
    },
  }

  try {
    const resp = await api.logsDockerContainerByNode(
      props.node_name,
      selectContainerId.value,
      {},
      on_container_logs_chunk,
      logsState.value.cancel.signal,
    )
    if (resp.status !== 200) {
      ElMessage.error('获取日志失败')
      return
    }
  } catch {
    logsState.value.cancel?.abort()
    logsState.value.cancel = null
  }
}

const restart_container = async () => {
  const { code, msg } = await (
    await api.restartDockerContainerByNode(
      props.node_name,
      selectContainerId.value,
    )
  ).data
  if (code !== 0) {
    ElMessage.error(msg || '重启容器失败')
    return
  }

  ElMessage.success('重启容器成功')
  refresh()
}

const start_container = async () => {
  const { code, msg } = await (
    await api.startDockerContainerByNode(
      props.node_name,
      selectContainerId.value,
    )
  ).data
  if (code !== 0) {
    ElMessage.error(msg || '启动容器失败')
    return
  }

  ElMessage.success('启动容器成功')
  refresh()
}

const stop_container = async () => {
  const { code, msg } = await (
    await api.stopDockerContainerByNode(
      props.node_name,
      selectContainerId.value,
    )
  ).data
  if (code !== 0) {
    ElMessage.error(msg || '停止容器失败')
    return
  }

  ElMessage.success('停止容器成功')
  refresh()
}

const get_docker_container_list = async node_name => {
  const { code, data, msg } = await (
    await api.getDockerContainerListByNode(node_name)
  ).data
  if (code !== 0) {
    ElMessage.error(msg || '获取容器列表失败')
    return
  }

  containerList.value = data.map(item => {
    return {
      id: item.Id.substring(0, 12),
      name: item.Names[0].substring(1),
      image: item.Image,
      stateTag: item.State === 'running' ? 'success' : 'danger',
      state: item.State,
      status: item.Status,
    }
  })
}

const on_open_control = id => {
  selectContainerId.value = id
  openControl.value = true
  currentContainerInfo.value = null

  // get container info
  inspect_container()
}

onMounted(() => {
  refresh()
})

defineExpose({
  refresh,
})
</script>

<style scoped>
.container-code {
  height: 86vh;
  overflow: hidden;
  display: inline-block;
  width: 100%;
}
</style>
