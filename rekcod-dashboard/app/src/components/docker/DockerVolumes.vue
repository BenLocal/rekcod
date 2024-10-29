<template>
  <el-table :data="volumes" style="width: 100%">
    <el-table-column prop="name" label="名称" width="180" />
    <el-table-column prop="driver" label="Driver" />
    <el-table-column prop="mount" label="挂载" />
  </el-table>
</template>

<script setup>
import { onMounted, ref } from 'vue'
import api from '../../api'
import { ElMessage } from 'element-plus'

const props = defineProps({
  node_name: String,
})
const volumes = ref([])

const refresh = async () => {
  await get_docker_volumes(props.node_name)
}

const get_docker_volumes = async node_name => {
  const { code, data, msg } = await (
    await api.getDockerVolumeListByNode(node_name)
  ).data
  if (code !== 0) {
    ElMessage.error(msg || '获取存储卷列表失败')
    return
  }
  if (!data.Volumes || data.Volumes.length === 0) {
    ElMessage.error('没有可用的存储卷')
    return
  }

  volumes.value = data.Volumes.map(item => {
    return {
      name: item.Name.substring(0, 12),
      driver: item.Driver || '-',
      mount: item.Mountpoint || '-',
    }
  })
}

onMounted(() => {
  refresh()
})
</script>
