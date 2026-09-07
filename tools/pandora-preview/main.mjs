import { createWorkerTransport, installLocalBootstrap } from './bootstrap.mjs';

const worker = new Worker('./worker.mjs', {type:'module', name:'pandora-preview'});
const transport = createWorkerTransport({worker, BlobCtor:Blob});
let lifecycle = null;

export const runtime = {
  request: (path, body) => transport.request(path, body),
  loadArt: () => transport.loadArt(),
  loadBackground: path => transport.loadBackground(path),
  replayForParity: actions => transport.replayForParity(actions),
  start({controller, loadArt, invalidatePresentation}) {
    lifecycle?.dispose();
    lifecycle = installLocalBootstrap({
      transport, controller, loadArt, invalidatePresentation, document, performance,
      memoryBytes: () => 0,
    });
    addEventListener('beforeunload', () => {
      lifecycle?.dispose();
      transport.terminate();
    }, {once:true});
  },
};
