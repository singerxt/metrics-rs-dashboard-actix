import {
  html,
  render,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "https://esm.sh/htm/preact/standalone";
import MetricBuffer from "./common/MetricBuffer.js";
import PrometheusImport from "./common/PrometheusImport.js";
import debounce from "./common/debounce.js";

// Constants for debounce timing
const REFRESH_DEBOUNCE_MS = 500;
const SEARCH_DEBOUNCE_MS = 300;
const METRIC_BUFFER_SIZE = 10;
const METRIC_ENDPOINT = "./metrics";

// Create singleton instances outside of the component to avoid recreating on each render
const metricBuffer = new MetricBuffer(METRIC_BUFFER_SIZE);
const prometheusImporter = new PrometheusImport(METRIC_ENDPOINT);

function Charts({ searchValue, refreshRate }) {
  return html`
    <div class="container">
      searchValue ${searchValue}
      refreshRate ${refreshRate}
    </div>
  `;
}

function App(props) {
  const [refreshRate, setRefreshRate] = useState(1000);
  const [searchValue, setSearchValue] = useState("");
  const [debouncedRefreshRate, setDebouncedRefreshRate] =
    useState(refreshRate);
  const [debouncedSearchValue, setDebouncedSearchValue] =
    useState(searchValue);

  // Use useMemo for expensive calculations that depend on specific inputs
  const debouncedSetRefreshRate = useMemo(
    () =>
      debounce((value) => {
        setDebouncedRefreshRate(value);
      }, REFRESH_DEBOUNCE_MS),
    [] // Empty dependency array means this is created only once
  );

  const debouncedSetSearchValue = useMemo(
    () =>
      debounce((value) => {
        setDebouncedSearchValue(value);
      }, SEARCH_DEBOUNCE_MS),
    [] // Empty dependency array means this is created only once
  );

  // Use a more efficient event handler
  const handleRefreshRateChange = useCallback((e) => {
    const value = Number.parseInt(e.target.value) || 0;
    setRefreshRate(value);
  }, []);

  const handleSearchChange = useCallback((e) => {
    setSearchValue(e.target.value);
  }, []);

  useEffect(() => {
    debouncedSetRefreshRate(refreshRate);
  }, [refreshRate, debouncedSetRefreshRate]);

  useEffect(() => {
    debouncedSetSearchValue(searchValue);
  }, [searchValue, debouncedSetSearchValue]);

  return html`
    <div class="container">
      <h1>Metrics</h1>
      <section class="grid">
        <input
          type="text"
          name="filter-by-prefix"
          placeholder="Filter by prefix"
          aria-label="Text"
          value=${searchValue}
          onInput=${handleSearchChange}
        />
        <input
          type="number"
          name="refresh-rate"
          placeholder="Refresh rate (ms)"
          aria-label="Number"
          value=${refreshRate}
          onInput=${handleRefreshRateChange}
        />
      </section>
      <section>
        <${Charts}
          searchValue=${debouncedSearchValue}
          refreshRate=${debouncedRefreshRate}
        />
      </section>
    </div>
  `;
}

render(html`<${App} name="World" />`, document.body);
