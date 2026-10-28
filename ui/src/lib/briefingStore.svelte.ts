// Which briefing is open, if any. The live plan's briefing is fetched on
// demand (it is large); saved flights carry theirs.
import { api, type FlightPlan, type Ofp } from "./api";

export const briefing = $state<{ plan: FlightPlan | null; ofp: Ofp | null; loading: boolean; error: string | null }>({
  plan: null,
  ofp: null,
  loading: false,
  error: null,
});

export async function openBriefing(plan: FlightPlan) {
  briefing.error = null;
  if (plan.ofp) {
    briefing.plan = plan;
    briefing.ofp = plan.ofp;
    return;
  }
  briefing.loading = true;
  try {
    let ofp = await api.currentBriefing();
    // A plan loaded before briefings existed: fetch it again from SimBrief.
    if (!ofp && plan.source === "simbrief") {
      const latest = await api.simbriefLatest();
      if (latest.ofp && (latest.id === plan.id || !plan.id)) {
        await api.setPlan(latest);
        ofp = latest.ofp;
      }
    }
    if (ofp) {
      briefing.plan = plan;
      briefing.ofp = ofp;
    } else {
      briefing.error = "This plan has no briefing. Import it again from SimBrief.";
    }
  } catch (e) {
    briefing.error = String(e);
  } finally {
    briefing.loading = false;
  }
}

export function closeBriefing() {
  briefing.plan = null;
  briefing.ofp = null;
}
