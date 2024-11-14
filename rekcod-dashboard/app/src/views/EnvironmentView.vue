<template>
  <dev class="container-code">
    <Codemirror
      ref="envRef"
      :value="envState.txt"
      :options="envState.options"
      border
    ></Codemirror>
  </dev>
  <dev>
    <el-button type="primary" @click="save_env">保存</el-button>
  </dev>
</template>

<script setup>
import { ref, onMounted, shallowRef } from 'vue'
import Codemirror from 'codemirror-editor-vue3'
import api from '../api'
import { ElMessage } from 'element-plus'

const envRef = shallowRef()
const envState = ref({
  txt: '',
  options: {
    mode: 'text/x-properties',
    lineNumbers: true,
    autoRefresh: true,
    scrollbarStyle: 'native',
  },
})

const save_env = async () => {
  const value = envRef.value?.cminstance.getValue()
  if (!value) {
    ElMessage.error('请输入环境变量')
    return
  }

  const { code, msg } = await (await api.saveEnv({ values: value })).data
  if (code !== 0) {
    ElMessage.error(msg || '保存失败')
    return
  }
  ElMessage.success('保存成功')
}

const get_env = async () => {
  const { code, data, msg } = await (await api.getEnv()).data
  if (code !== 0) {
    ElMessage.error(msg || '获取系统信息失败')
    return
  }

  envState.value.txt = data.values
}

onMounted(() => {
  get_env()
})
</script>
