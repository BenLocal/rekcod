import axios from 'axios'

const api = {
  async getNodeList(data) {
    return axios.post('/api/node/list', data)
  },
  async getDockerInfoByNode(nodeName, data) {
    return axios.post(`/api/node/docker/info?node_name=${nodeName}`, data)
  },
  async getDockerContainerListByNode(nodeName, data) {
    return axios.post(
      `/api/node/docker/container/list?node_name=${nodeName}`,
      data,
    )
  },
  async stopDockerContainerByNode(nodeName, id, data) {
    return axios.post(
      `/api/node/docker/container/stop/${id}?node_name=${nodeName}`,
      data,
    )
  },
  async restartDockerContainerByNode(nodeName, id, data) {
    return axios.post(
      `/api/node/docker/container/restart/${id}?node_name=${nodeName}`,
      data,
    )
  },
  async startDockerContainerByNode(nodeName, id, data) {
    return axios.post(
      `/api/node/docker/container/start/${id}?node_name=${nodeName}`,
      data,
    )
  },
  async removeDockerContainerByNode(nodeName, id, data) {
    return axios.post(
      `/api/node/docker/container/delete/${id}?node_name=${nodeName}`,
      data,
    )
  },
  async inspectDockerContainerByNode(nodeName, id, data) {
    return axios.post(
      `/api/node/docker/container/inspect/${id}?node_name=${nodeName}`,
      data,
    )
  },
  async logsDockerContainerByNode(nodeName, id, data, postrogress, signal) {
    return axios.post(
      `/api/node/docker/container/logs/${id}?node_name=${nodeName}`,
      data,
      {
        responseType: 'stream',
        headers: {
          'Content-Type': 'application/json',
        },
        onDownloadProgress: progressEvent => {
          const datachunk = progressEvent.event.currentTarget.response
          postrogress && postrogress(datachunk)
        },
        signal: signal,
      },
    )
  },
  async getDockerImageListByNode(node, data) {
    return axios.post(`/api/node/docker/image/list?node_name=${node}`, data)
  },
  async getDockerNetworkListByNode(node, data) {
    return axios.post(`/api/node/docker/network/list?node_name=${node}`, data)
  },
  async getDockerVolumeListByNode(node, data) {
    return axios.post(`/api/node/docker/volume/list?node_name=${node}`, data)
  },
  async getNodeSysInfo(node) {
    return axios.get('/api/node/proxy/sys', {
      headers: {
        'X-NODE-NAME': node,
      },
    })
  },
  async getAppTmplList() {
    return axios.post('/api/app/tmpl/list')
  },
  async getAppTmplInfo(id) {
    return axios.post(`/api/app/tmpl/info/${id}`)
  },
  async getEnv() {
    return axios.post('/api/env/list')
  },
  async saveEnv(data) {
    return axios.post('/api/env/set', data)
  },
  async deploy(data) {
    // value is yml string
    return axios.post('/api/app/deploy', data)
  }
}

export default api
