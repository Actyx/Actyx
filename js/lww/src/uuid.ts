import { v4 } from 'uuid'
import { UniqueId } from './base'

export const mkUniqueId = () => v4() as UniqueId
