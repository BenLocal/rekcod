<template>
  <el-row class="tac" style="margin-bottom: 15px">
    <el-col :span="12">
      <el-link type="primary" @click="$router.push(`/node`)"
        >Node: &nbsp;
      </el-link>
      {{ node_name }}
    </el-col>
  </el-row>
  <el-row style="margin-bottom: 15px" justify="space-around">
    <el-col :span="6">
      <el-card class="box-card" body-class="card-body-center">
        <template #header>
          <span>CPU</span>
        </template>
        <el-progress type="circle" :percentage="sysInfo.cpu_usage" />
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card class="box-card" body-class="card-body-center">
        <template #header>
          <span>Mem</span>
        </template>
        <el-progress type="circle" :percentage="sysInfo.mem_usage" />
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card class="box-card" body-class="card-body-center">
        <template #header>
          <span>Disks</span>
        </template>
        <div
          class="disk-progress"
          v-if="sysInfo.disks && sysInfo.disks.length > 0"
        >
          <el-progress
            v-for="disk in sysInfo.disks"
            :key="disk.device"
            :text-inside="true"
            :stroke-width="26"
            :percentage="disk.percent"
            striped
            striped-flow
            :duration="50"
            :status="disk.status"
          />
        </div>
        <div v-else>
          <el-progress type="circle" :percentage="0" status="exception" />
        </div>
      </el-card>
    </el-col>
  </el-row>
  <el-row>
    <el-tabs
      v-model="selected"
      type="border-card"
      closeable="false"
      @tab-change="on_tab_change"
      style="width: 100%"
    >
      <el-tab-pane
        v-for="item in tabItems"
        :key="item.name"
        :label="item.label"
        :name="item.name"
      >
        <component v-if="selected == item.name" :is="item.component" :node_name="node_name"></component>
      </el-tab-pane>
    </el-tabs>
  </el-row>
</template>

<script setup>
import { onMounted, ref, shallowRef } from 'vue'
import DockerContainers from '../components/docker/DockerContainers.vue'
import DockerInfo from '../components/docker/DockerInfo.vue'
import DockerImages from '../components/docker/DockerImages.vue'
import DockerNetworks from '../components/docker/DockerNetworks.vue'
import DockerVolumes from '../components/docker/DockerVolumes.vue'
import api from '../api'
import { ElMessage } from 'element-plus'

const props = defineProps({ node_name: String })
const selected = ref('tab-1')
const tabItems = ref([
  {
    label: 'Containers',
    name: 'tab-1',
    component: shallowRef(DockerContainers),
  },
  {
    label: 'Info',
    name: 'tab-2',
    component: shallowRef(DockerInfo),
  },
  {
    label: 'Images',
    name: 'tab-3',
    component: shallowRef(DockerImages),
  },
  {
    label: 'Networks',
    name: 'tab-4',
    component: shallowRef(DockerNetworks),
  },
  {
    label: 'Volumes',
    name: 'tab-5',
    component: shallowRef(DockerVolumes),
  },
])
const sysInfo = ref({
  cpu: 0,
  mem: 0,
  disk: [],
})

const on_tab_change = () => {
  console.log(selected.value)
}

const get_sys = async () => {
  const { code, data, msg } = await (
    await api.getNodeSysInfo(props.node_name)
  ).data
  if (code !== 0) {
    ElMessage.error(msg || '获取系统信息失败')
    return
  }
  console.log(data)

  let disks = data.disks?.map(item => {
        const p = (((item.total - item.free) / item.total) * 100).toFixed(2)
        const status = p > 80 ? 'danger' : p > 60 ? 'warning' : 'success'
        return {
          name: item.name,
          percent: p,
          status: status,
        }
      }) || []
  if (disks.length > 0) {
    disks = disks.slice(0, 3)
  }
  sysInfo.value = {
    cpu_usage: data.cpu_usage.toFixed(2),
    mem_usage: data.mem_usage.toFixed(2),
    disks: disks,
  }
}

onMounted(() => {
  get_sys()
})
</script>

<style>
.card-body-center {
  display: flex;
  justify-content: center;
  align-items: center;
}

.disk-progress {
  width: 100%;
  height: 100%;
}

.disk-progress .el-progress--line {
  margin-bottom: 15px;
  width: 100%;
  height: 100%;
}
</style>
