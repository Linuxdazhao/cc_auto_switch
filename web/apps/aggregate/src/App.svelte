<script lang="ts">
  import { onMount } from "svelte";
  import { Tabs } from "@ccs/ui";
  import Header from "./components/Header.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import RequestsView from "./views/RequestsView.svelte";
  import SessionsView from "./views/SessionsView.svelte";
  import { state, loadAll, loadRequests } from "./store.svelte";

  onMount(async () => {
    await loadAll();
    await loadRequests();
  });
</script>

<div class="flex h-screen flex-col">
  <Header />
  <div class="flex flex-1 overflow-hidden">
    <Sidebar />
    <main class="flex-1 overflow-y-auto p-4">
      <Tabs.Root bind:value={state.view}>
        <Tabs.List>
          <Tabs.Trigger value="requests">Requests</Tabs.Trigger>
          <Tabs.Trigger value="sessions">Sessions</Tabs.Trigger>
        </Tabs.List>
        <Tabs.Content value="requests"><RequestsView /></Tabs.Content>
        <Tabs.Content value="sessions"><SessionsView /></Tabs.Content>
      </Tabs.Root>
    </main>
  </div>
</div>
