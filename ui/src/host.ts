// The page: the grant arrives in the fragment from ./session (after the apps.ce-net.com sign-in), is kept in memory
// only and wiped from the address bar; then ui-host asks this viewer's manifest and mounts what it lists.
import { grantFromLink } from '../gen/CrShell';
import { liveBoot, start } from '@ui/host';

const grant = grantFromLink();
const app = document.getElementById('app');
if (grant === null) {
  location.replace('session');
} else if (app !== null) {
  history.replaceState(null, '', location.pathname);
  start(liveBoot(grant), app).catch((e: unknown) => {
    app.textContent = `This page could not start: ${e instanceof Error ? e.message : String(e)}`;
  });
}
