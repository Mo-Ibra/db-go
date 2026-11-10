import { useDatabaseSession } from "@/features/connection/hooks/useDatabaseSession";
import { DatabaseWorkspace } from "@/features/workspace/components/DatabaseWorkspace";
import { WelcomeScreen } from "@/screens/welcome/WelcomeScreen";

export default function App() {
  const session = useDatabaseSession();

  if (!session.db) {
    return <WelcomeScreen session={session} />;
  }

  return (
    <DatabaseWorkspace
      key={session.db.path}
      session={session}
    />
  );
}
