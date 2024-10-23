import axios from 'axios'

const api = {
  async getNodeList(data) {
    return axios.post('/api/node/list', data)
  },
}

export default api
