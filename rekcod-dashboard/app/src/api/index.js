import axios from 'axios'

const api = {
    async getNodeList(data) {
        return axios.post('/api/node/list', data)
    },
    async getDockerInfoByNode(nodeName, data) {
        return axios.post(`/api/node/docker/info?node_name=${nodeName}`, data)
    },
    async getDockerContainerListByNode(nodeName, data) {
        return axios.post(`/api/node/docker/container/list?node_name=${nodeName}`, data)
    },
    async stopDockerContainerByNode(nodeName, id, data) {
        return axios.post(`/api/node/docker/container/stop/${id}?node_name=${nodeName}`, data)
    },
    async restartDockerContainerByNode(nodeName, id, data) {
        return axios.post(`/api/node/docker/container/restart/${id}?node_name=${nodeName}`, data)
    },
    async startDockerContainerByNode(nodeName, id, data) {
        return axios.post(`/api/node/docker/container/start/${id}?node_name=${nodeName}`, data)
    },
    async removeDockerContainerByNode(nodeName, id, data) {
        return axios.post(`/api/node/docker/container/delete/${id}?node_name=${nodeName}`, data)
    },
    async inspectDockerContainerByNode(nodeName, id, data) {
        return axios.post(`/api/node/docker/container/inspect/${id}?node_name=${nodeName}`, data)
    },
    async logsDockerContainerByNode(nodeName, id, data, postrogress, signal) {
        return axios.post(`/api/node/docker/container/logs/${id}?node_name=${nodeName}`, data, {
            responseType: 'stream',
            headers: {
                'Content-Type': 'application/json'
            },
            onDownloadProgress: (progressEvent) => {
                const datachunk = progressEvent.event.currentTarget.response
                postrogress && postrogress(datachunk)
            },
            signal: signal
        })
    }
}

export default api
