<template>
  <el-table :data="networks" style="width: 100%">
    <el-table-column prop="id" label="id" width="180" />
    <el-table-column prop="name" label="名称" width="180" />
    <el-table-column prop="driver" label="Driver" />
    <el-table-column prop="config" label="网络" />
    <el-table-column prop="gate" label="网关" />
  </el-table>
</template>

<script setup>
import { onMounted, ref } from 'vue'
import api from '../../api'
import { ElMessage } from 'element-plus'

const props = defineProps({
  node_name: String,
})
const networks = ref([])

const refresh = async () => {
  await get_docker_networks(props.node_name)
}

const get_docker_networks = async node_name => {
  const { code, data, msg } = await (
    await api.getDockerNetworkListByNode(node_name)
  ).data
  if (code !== 0) {
    ElMessage.error(msg || '获取网络列表失败')
    return
  }

  networks.value = data.map(item => {
    return {
      id: item.Id.replace('sha256:', '').substring(0, 12),
      name: item.Name,
      driver: item.Driver || '-',
      config: item.IPAM?.Config[0]?.Subnet || '-',
      gate: item.IPAM?.Config[0]?.Gateway || '-',
    }
  })
}

onMounted(() => {
  refresh()
})
</script>
