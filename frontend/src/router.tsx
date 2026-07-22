import { createBrowserRouter } from 'react-router-dom'
import { AppShell } from '@/components/layout/AppShell'
import { RequireAccess } from '@/components/layout/RequireAccess'
import { AdminProjectsPage } from '@/pages/AdminProjectsPage'
import { HomePage } from '@/pages/HomePage'
import { LoginPage } from '@/pages/LoginPage'
import { NotFoundPage } from '@/pages/NotFoundPage'
import { ProjectDetailPage } from '@/pages/ProjectDetailPage'
import { ProjectEditorPage } from '@/pages/ProjectEditorPage'
import { ProjectsPage } from '@/pages/ProjectsPage'

export const router: ReturnType<typeof createBrowserRouter> = createBrowserRouter([
  { path: '/login', element: <LoginPage /> },
  {
    element: <AppShell />,
    children: [
      { path: '/', element: <HomePage /> },
      { path: '/projects', element: <ProjectsPage /> },
      { path: '/projects/:slug', element: <ProjectDetailPage /> },
      {
        path: '/admin/projects',
        element: (
          <RequireAccess requireLabMember>
            <AdminProjectsPage />
          </RequireAccess>
        ),
      },
      {
        path: '/admin/projects/new',
        element: (
          <RequireAccess requireLabMember>
            <ProjectEditorPage />
          </RequireAccess>
        ),
      },
      {
        path: '/admin/projects/:slug/edit',
        element: (
          <RequireAccess requireLabMember>
            <ProjectEditorPage />
          </RequireAccess>
        ),
      },
      { path: '*', element: <NotFoundPage /> },
    ],
  },
])
