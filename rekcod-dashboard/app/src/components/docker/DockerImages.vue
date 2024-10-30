<template>
  <el-table :data="images" style="width: 100%">
    <el-table-column prop="id" label="id" width="180" />
    <el-table-column prop="name" label="名称" width="180" />
    <el-table-column prop="tag" label="TAG" />
    <el-table-column prop="container_count" label="使用容器个数" />
  </el-table>
</template>

<script setup>
import { onMounted, ref } from 'vue'
import api from '../../api'
import { ElMessage } from 'element-plus'

const props = defineProps({
  node_name: String,
})
const images = ref([])

const refresh = async () => {
  await get_docker_images_list(props.node_name)
}

const get_docker_images_list = async node_name => {
  const { code, data, msg } = await (
    await api.getDockerImageListByNode(node_name)
  ).data
  if (code !== 0) {
    ElMessage.error(msg || '获取镜像列表失败')
    return
  }

  images.value = data.map(item => {
    let name = "<none>"
    let tag = "<none>"
    if (item.RepoTags && item.RepoTags.length > 0) {
      name = item.RepoTags[0].split(':')[0] || "<none>"
      tag = item.RepoTags[0].split(':')[1] || "<none>"
    }

    return {
      id: item.Id.replace('sha256:', '').substring(0, 12),
      name: item.RepoTags[0].split(':')[0],
      tag: item.RepoTags[0].split(':')[1],
      container_count: item.Containers < 0 ? 0 : item.Containers,
    }
  })
}

onMounted(() => {
  refresh()
})
</script>
