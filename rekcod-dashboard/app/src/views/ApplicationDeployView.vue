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
import { ElMessage } from 'element-plus'
import { onMounted, ref, watch } from 'vue'
import { Document } from 'yaml'
import api from '../api'

const props = defineProps({ id: String })
const appInfo = ref({
  id: '',
  app_name: '',
  node_name: '',
  build: false,
  qa: [],
})
const nodes = ref([])
const valuesYml = ref('')

const get_app_tmpl_info = async id => {
  const { code, data, msg } = await (await api.getAppTmplInfo(id)).data
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

const on_deploy_submit = async () => {
  console.log(valuesYml.value)
  const data = {
    id: appInfo.value.id,
    app_name: appInfo.value.app_name,
    node_name: appInfo.value.node_name,
    values: valuesYml.value,
  }
  console.log(data)
  const { code, msg } = await (await api.deploy(data)).data
  if (code !== 0) {
    ElMessage.error(msg || '部署失败')
    return
  }

  ElMessage.success('部署成功')
}

watch(appInfo, (n) => {
  console.log(n)
  if (!n || !n.qa || n.qa.length === 0) {
    valuesYml.value = ''
    return
  }

  let obj = {}
  n.qa.forEach(item => {
    obj[item.name] = item.value
  })
  const doc = new Document();
  doc.contents = obj
  valuesYml.value = doc.toString()
}, { deep: true })

onMounted(() => {
  get_node_list()
  get_app_tmpl_info(props.id)
})
</script>
