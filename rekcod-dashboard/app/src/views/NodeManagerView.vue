<template>
  <el-table :data="tableData" height="250" style="width: 100%">
    <el-table-column label="节点名" width="180">
      <template #default="scope">
        <el-link type="primary" @click="goto_docker_view(scope.row.name)"
          >{{ scope.row.name }}
        </el-link>
      </template>
    </el-table-column>
    <el-table-column prop="host" label="HOST" />
    <el-table-column label="状态">
      <template #default="scope">
        <el-tag :type="scope.row.status ? 'success' : 'danger'">{{
          scope.row.statusText
        }}</el-tag>
      </template>
    </el-table-column>
  </el-table>
</template>

<script setup>
import { onMounted, ref } from 'vue'
import api from '../api'
import { ElMessage } from 'element-plus'
import { useRouter } from 'vue-router'

const router = useRouter()

const tableData = ref([])

const goto_docker_view = (node_name) => {
  router.push({ path: '/node/docker', query: { node_name: node_name } })
}

onMounted(async () => {
  const { code, data, msg } = await (await api.getNodeList({ all: true })).data
  if (code !== 0) {
    ElMessage.error(msg ? msg : '获取节点列表失败')
    return
  }
  const res = data.map(item => {
    return {
      name: item.name,
      host: item.ip + ':' + item.port,
      status: item.status,
      statusText: item.status ? '在线' : '离线',
    }
  })

  tableData.value = res
})

</script>
<style scoped></style>
