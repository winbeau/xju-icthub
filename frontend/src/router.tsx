import { createBrowserRouter } from 'react-router-dom'
import { AppShell } from '@/components/layout/AppShell'
import { RequireAccess } from '@/components/layout/RequireAccess'
import { AdminProjectsPage } from '@/pages/AdminProjectsPage'
import { HomePage } from '@/pages/HomePage'
import { ImportProjectPage } from '@/pages/ImportProjectPage'
import { LoginPage } from '@/pages/LoginPage'
import { NotFoundPage } from '@/pages/NotFoundPage'
import { ProjectDetailPage } from '@/pages/ProjectDetailPage'
import { ProjectCreatePage } from '@/pages/ProjectCreatePage'
import { ProjectEditorPage } from '@/pages/ProjectEditorPage'
import { ProjectsPage } from '@/pages/ProjectsPage'
import { TagManagementPage } from '@/pages/TagManagementPage'

export const router: ReturnType<typeof createBrowserRouter> = createBrowserRouter([
  { path: '/login', element: <LoginPage /> },
  {
    element: (
      <RequireAccess requireLabMember>
        <AppShell />
      </RequireAccess>
    ),
    children: [
      { path: '/', element: <HomePage /> },
      { path: '/projects', element: <ProjectsPage /> },
      { path: '/projects/:slug', element: <ProjectDetailPage /> },
      { path: '/admin/projects', element: <AdminProjectsPage /> },
      { path: '/admin/import', element: <ImportProjectPage /> },
      { path: '/admin/projects/new', element: <ProjectCreatePage /> },
      { path: '/admin/projects/:slug/edit', element: <ProjectEditorPage /> },
      { path: '/admin/tags', element: <TagManagementPage /> },
      { path: '*', element: <NotFoundPage /> },
    ],
  },
])
