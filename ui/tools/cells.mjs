// The site's cells (repo:control-room crates/control-room-site), for the build and the lint: repo, contract, api
// ident, the view's bundle key and its source. The cell repos sit beside this one (~/dev/<repo>).
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const dev = resolve(repo, '..');
export const CELLS = [
  { repo, contract: 'crates/cr-shell-api/src/lib.rs', ident: 'CrShell', key: 'cr.shell@2/ControlRoom', view: 'ui/src/ControlRoom.ts' },
  { repo: join(dev, 'cr-keys'), contract: 'crates/cr-keys-api/src/lib.rs', ident: 'CrKeys', key: 'cr.keys@2/KeyList', view: 'ui/src/KeyList.ts' },
  { repo: join(dev, 'cr-devices'), contract: 'crates/cr-devices-api/src/lib.rs', ident: 'CrDevices', key: 'cr.devices@2/DeviceList', view: 'ui/src/DeviceList.ts' },
  { repo: join(dev, 'cr-audit'), contract: 'crates/cr-audit-api/src/lib.rs', ident: 'CrAudit', key: 'cr.audit@2/AuditTrail', view: 'ui/src/AuditTrail.ts' },
  { repo: join(dev, 'sub-gateway'), contract: 'crates/sub-gateway-api/src/lib.rs', ident: 'SubGateway', key: 'sub.gateway@1/QuotaBoard', view: 'ui/src/QuotaBoard.ts' },
];

