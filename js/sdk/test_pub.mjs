import { Actyx, Tags } from './lib'
import { execSync } from 'child_process'

(async () => {
	const a = await Actyx.of({"appId":"com.example.x","displayName":"test","version":"42"})
	let l = 0
	while (true) {
		try {
			const m = await a.publish(Tags('testPub').apply("hello world" + l))
			// execSync('ax internal shutdown localhost', {env: {...process.env, HERE_BE_DRAGONS: 'zøg'}})
			if (m.lamport <= l) { console.error("***!!!***"); return }
			l = m.lamport
			console.log(m.lamport)
		} catch (e) {
			console.log(e)
		}
	}
})()
