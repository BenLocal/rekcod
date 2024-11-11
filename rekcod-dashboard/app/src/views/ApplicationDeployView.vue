<template>
  <div>
    <el-form :model="appInfo" label-width="auto" style="max-width: 600px">
      <el-form-item label="名称">
        <el-input v-model="appInfo.app_name" />
      </el-form-item>
      <el-form-item label="所属节点">
        <el-select v-model="appInfo.node_name">
          <el-option
            v-for="item in nodes"
            :key="item.value"
            :label="item.label"
            :value="item.value"
          />
        </el-select>
      </el-form-item>
      <el-form-item v-for="qa in appInfo.qa" :key="qa.id" :label="qa.label">
        <el-input v-model="qa.value" />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="on_deploy_submit">deploy</el-button>
        <el-button>cancel</el-button>
      </el-form-item>
    </el-form>
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue'
import api from '../api'
import { ElMessage } from 'element-plus'

const props = defineProps({ id: String })
const appInfo = ref({
  id: '',
  app_name: '',
  node_name: '',
  qa: [],
})
const nodes = ref([])

const get_app_info = async id => {
  const { code, data, msg } = await (await api.getAppInfo(id)).data
  if (code !== 0) {
    ElMessage.error(msg || '获取Application列表失败')
    return
  }

  appInfo.value = {
    id: data.id,
    app_name: data.name,
    node_name: data.node_name,
    qa: data.qa,
  }
}

const get_node_list = async () => {
  const { code, data, msg } = await (await api.getNodeList({ all: true })).data
  if (code !== 0) {
    ElMessage.error(msg || '获取节点列表失败')
    return
  }

  nodes.value = data
    .filter(item => item.status)
    .map(item => {
      return {
        value: item.name,
        label: item.name,
      }
    })
}

const on_deploy_submit = () => {}

onMounted(() => {
  get_node_list()
  get_app_info(props.id)
})
</script>
